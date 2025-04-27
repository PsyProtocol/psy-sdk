use crate::config::RealmNodeConfig;
use crate::{C, D, F};
use fred::prelude::KeysInterface;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq::traits::KVQSerializable;
use kvq_store_lmdbx::KVQlibmdbxStore;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::config::network_constants::QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL;
use qed_core::job::drain_queue::CheckpointDrainQueueEmitterAsyncImm;
use qed_core::job::history_queue::CheckpointHistoryQueueConsumerAsyncImm;
use qed_core::job::id::{ProvingJobDataId, QProvingJobDataID};
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_crypto::common::simple_circuit_library::SimpleCircuitLibrary;
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;
use qed_node::nimpl::new_fred_pool;
use qed_node::nimpl::proof_store_fred::{ProofStoreFred, PS_HISTORY_QUEUE_KEY_PREFIX};
use qed_node::realm::state::processor::{RealmConfig, RealmProcessorContext};
use qed_node::worker::simple_async_realm::SimpleAsyncRealmWorker;
use qed_node_common::verifier::get_cached_generic_verifier;
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info};

type KVQArcImmutableStore = KVQArcImmutableStoreWrapper<KVQlibmdbxStore>;
type ConcreteRealmProcessorContext = RealmProcessorContext<
    KVQArcImmutableStore,
    ProofStoreFred,
    ProofStoreFred,
    ProofStoreFred,
    ProofStoreFred,
>;

#[derive(Debug)]
pub struct RealmProcessor {
    pub realm_config: RealmConfig,
    pub queue: ProofStoreFred,
    pub store: KVQArcImmutableStore,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
    pub synced_checkpoint_id: u64,
}

pub async fn run_realm_processor(config: RealmNodeConfig) -> anyhow::Result<()> {
    let realm_processor = RealmProcessor::new(config).await?;
    let handle = realm_processor.start().await?;

    tokio::select! {
        _ = handle => {
            panic!("Realm processor stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl-C, shutting down...");
        }
    }
    Ok(())
}
pub const REALM_PROCESSOR_SUFFIX: &str = "RP";
impl RealmProcessor {
    pub async fn new(config: RealmNodeConfig) -> anyhow::Result<Self> {
        info!("Realm Processor Config: {:?}", config);
        let pool =
            new_fred_pool(&config.redis.redis_uri, config.redis.pool_size.unwrap_or(8)).await?;
        let realm_qps = ProofStoreFred::new2(
            pool,
            config.queue.worker_queue_suffix,
            config.queue.notifications_queue_suffix,
            Some(REALM_PROCESSOR_SUFFIX),
            Some(REALM_PROCESSOR_SUFFIX),
        );
        let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore> =
            KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(KVQlibmdbxStore::new_write(
                &config.db.path,
            )?);

        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

        let coordinator_worker_circuits =
            QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);

        let realm_config = RealmConfig::get_standard(config.realm.node_id, config.realm.realm_id);
        let processor = RealmProcessor {
            realm_config,
            queue: realm_qps,
            store: store_reader,
            proof_verifier,
            coordinator_worker_circuits,
            synced_checkpoint_id: 0,
        };
        Ok(processor)
    }

    pub async fn start(mut self) -> anyhow::Result<JoinHandle<()>> {
        info!("Realm Processor starting");
        let st = Arc::new(self.store.dup());
        let realm_qps = Arc::new(self.queue.clone());
        let mut context = if st.get_latest_l2_block_state().await.is_ok() {
            info!("Init state from database");
            RealmProcessorContext::new(
                self.realm_config,
                st.clone(),
                realm_qps.clone(),
                realm_qps.clone(),
                realm_qps.clone(),
                realm_qps.clone(),
                self.proof_verifier.clone(),
            )
            .await?
        } else {
            info!("start to init state from queue");
            let get_latest_checkpoint_id = get_latest_checkpoint_id(&self.queue)
                .await?
                .ok_or(anyhow::anyhow!("No latest checkpoint id found in queue"))?;
            info!("latest checkpoint id: {:?}", get_latest_checkpoint_id);
            let checkpoint = get_checkpoint(&self.queue, get_latest_checkpoint_id).await?;
            let l2_block_state = checkpoint.l2_block_state.clone();
            info!(?l2_block_state, "Init state from queue");
            RealmProcessorContext {
                store: st.clone(),
                checkpoint_queue: realm_qps.clone(),
                sync_queue: realm_qps.clone(),
                prover_queue: realm_qps.clone(),
                proof_store: realm_qps.clone(),
                proof_verifier: self.proof_verifier.clone(),
                latest_block_state: l2_block_state,
                realm_config: self.realm_config,
                pending_register_users: vec![],
            }
        };
        info!("Realm Processor started");
        let handle = tokio::spawn(async move {
            loop {
                info!("Waiting for next checkpoint");
                match self.sync_checkpoint(&mut context).await {
                    Ok(false) | Err(_) => {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                    _ => {}
                }
                info!("Start building block");
                let proving_data_job_id: ProvingJobDataId =
                    match self.build_block(&mut context).await {
                        Ok(job_id) => job_id,
                        Err(err) => {
                            error!("Error building block: {:?}", err);
                            continue;
                        }
                    };
                // Send the job id to the channel for the next step
                info!("Pushing job id to queue: {:?}", proving_data_job_id);
                if let Err(err) = self.queue.cdq_push_imm(proving_data_job_id).await {
                    error!("Error chq_push_imm: {:?}", err);
                };
                info!("Pushing job to queueue done");
            }
        });
        Ok(handle)
    }

    pub async fn sync_checkpoint(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
    ) -> anyhow::Result<bool> {
        let checkpoint = self.wait_for_next_checkpoint().await;
        match checkpoint {
            Ok(checkpoint) => {
                // checkpoint.l2_block_state
                let checkpoint_id = checkpoint.l2_block_state.checkpoint_id;
                info!(?checkpoint_id, "Checkpoint received: {:?}", checkpoint);
                match context.handle_checkpoint_sync(checkpoint).await {
                    Ok(_) => {
                        info!(?checkpoint_id, "Sync to new checkpoint");
                        self.synced_checkpoint_id = checkpoint_id;
                        Ok(true)
                    }
                    Err(err) => {
                        error!(?checkpoint_id, ?err, "Error sync checkpoint");
                        Err(err)
                    }
                }
            }
            Err(err) => {
                error!(?self.synced_checkpoint_id, "Error getting checkpoint sync info: {:?}", err);
                Err(err)
            }
        }
    }

    pub async fn build_block(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
    ) -> anyhow::Result<ProvingJobDataId> {
        context.build_block().await?;
        let realm_worker_output_job_id = self.run_worker_until_done().await?;
        let checkpoint_id = self.synced_checkpoint_id + 1;
        Ok(ProvingJobDataId::new(
            checkpoint_id,
            realm_worker_output_job_id,
        ))
    }

    pub async fn run_worker_until_done(&self) -> anyhow::Result<QProvingJobDataID> {
        SimpleAsyncRealmWorker::run_worker_until_done::<
            _,
            _,
            SimpleCircuitLibrary<GoldilocksField>,
            QEDCoordinatorCircuitManager<C, D>,
            C,
            D,
        >(
            &self.queue.clone(),
            &self.queue.clone(),
            &self.coordinator_worker_circuits,
            &self.proof_verifier.library,
        )
        .await
    }

    pub async fn wait_for_next_checkpoint(
        &self,
    ) -> anyhow::Result<QEDCheckpointSyncInfoCompact<F>> {
        let next_checkpoint_id = self.synced_checkpoint_id + 1;
        self.queue
            .wait_for_next_item_imm(
                QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL,
                next_checkpoint_id,
            )
            .await
    }
}

async fn get_latest_checkpoint_id(queue: &ProofStoreFred) -> anyhow::Result<Option<u64>> {
    queue
        .current_checkpoint_id(QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL)
        .await
}

async fn get_checkpoint(
    queue: &ProofStoreFred,
    checkpoint_id: u64,
) -> anyhow::Result<QEDCheckpointSyncInfoCompact<F>> {
    let result = queue
        .pool()
        .get::<Vec<u8>, String>(format!(
            "{}-{}_{}",
            PS_HISTORY_QUEUE_KEY_PREFIX,
            QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL,
            checkpoint_id,
        ))
        .await?;
    Ok(QEDCheckpointSyncInfoCompact::from_bytes(&result)?)
}
