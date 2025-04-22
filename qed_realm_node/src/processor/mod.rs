use crate::config::RealmNodeConfig;
use crate::{new_store_reader, new_with_connection, C, D, F};
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::job::history_queue::CheckpointHistoryQueueEmitterAsyncImm;
use qed_core::job::id::{ProvingJobDataId, QProvingJobDataID};
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_crypto::common::simple_circuit_library::SimpleCircuitLibrary;
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::realm::state::processor::{RealmConfig, RealmProcessorContext};
use qed_node::worker::simple_async_realm::SimpleAsyncRealmWorker;
use qed_node_common::verifier::get_cached_generic_verifier;
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_store::node::coordinator::store_traits::QEDCoordinatorStoreReaderAsync;
use reth_libmdbx::{RW};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info};

type KVQArcImmutableStore = KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>>;
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
    pub realm_qps: ProofStoreFred,
    pub store_reader: KVQArcImmutableStore,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
    pub synced_checkpoint_id: u64,
}

pub async fn start_realm_processor_node(config: RealmNodeConfig) -> anyhow::Result<()> {
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

impl RealmProcessor {
    pub async fn new(config: RealmNodeConfig) -> anyhow::Result<Self> {
        let pool = new_with_connection(&config.redis.url,config.redis.pool_size.unwrap_or(8)).await?;
        let realm_qps = ProofStoreFred::new(
            pool,
            config.queue.worker_queue_suffix,
            config.queue.notifications_queue_suffix,
        );
        let store_reader = new_store_reader(&config.db.path).await?;
        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

        let coordinator_worker_circuits =
            QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);

        let realm_config = RealmConfig::get_standard(config.realm.node_id, config.realm.realm_id);
        let processor = RealmProcessor {
            realm_config,
            realm_qps,
            store_reader,
            proof_verifier,
            coordinator_worker_circuits,
            synced_checkpoint_id: 0,
        };
        Ok(processor)
    }

    pub async fn start(mut self) -> anyhow::Result<JoinHandle<()>> {
        let st = Arc::new(self.store_reader.dup());
        let realm_qps = Arc::new(self.realm_qps.clone());
        let mut context = RealmProcessorContext::new(
            self.realm_config,
            st.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            self.proof_verifier.clone(),
        )
        .await?;
        let handle = tokio::spawn(async move {
            loop {
                match self.sync_checkpoint(&mut context).await {
                    Ok(false) | Err(_) => {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                    _ => {}
                }
                let proving_data_job_id: ProvingJobDataId =
                    match self.build_block(&mut context).await {
                        Ok(job_id) => job_id,
                        Err(err) => {
                            error!("Error building block: {:?}", err);
                            continue;
                        }
                    };
                // Send the job id to the channel for the next step
                if let Err(err) = self.realm_qps.chq_push_imm(proving_data_job_id).await {
                    error!("Error chq_push_imm: {:?}", err);
                };
            }
        });
        Ok(handle)
    }

    pub async fn sync_checkpoint(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
    ) -> anyhow::Result<bool> {
        let newer_checkpoint_id = match self.checkpoint_id().await {
            Ok(checkpoint_id) if self.synced_checkpoint_id >= checkpoint_id => {
                info!("Checkpoint id not updated");
                return Ok(false);
            }
            Ok(checkpoint_id) => {
                info!("Find newer checkpoint id: {:?}", checkpoint_id);
                checkpoint_id
            }
            Err(err) => {
                error!("Error getting checkpoint id: {:?}", err);
                return Err(err);
            }
        };
        match self
            .store_reader
            .get_checkpoint_sync_info_compact(newer_checkpoint_id)
            .await
        {
            Ok(checkpoint) => {
                info!(
                    ?newer_checkpoint_id,
                    "Checkpoint received: {:?}", checkpoint
                );
                self.synced_checkpoint_id = checkpoint.l2_block_state.checkpoint_id;
                match context.handle_checkpoint_sync(checkpoint).await {
                    Ok(_) => {
                        info!(?newer_checkpoint_id, "Sync to new checkpoint");
                        self.synced_checkpoint_id = newer_checkpoint_id;
                        Ok(true)
                    }
                    Err(err) => {
                        error!(?newer_checkpoint_id, ?err, "Error sync checkpoint");
                        Err(err)
                    }
                }
            }
            Err(err) => {
                error!("Error getting checkpoint sync info: {:?}", err);
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
        let checkpoint_id = self.synced_checkpoint_id;
        Ok(ProvingJobDataId::new(
            checkpoint_id,
            realm_worker_output_job_id,
        ))
    }

    pub async fn checkpoint_id(&self) -> anyhow::Result<u64> {
        Ok(self
            .store_reader
            .get_latest_l2_block_state()
            .await?
            .checkpoint_id)
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
            &self.realm_qps.clone(),
            &self.realm_qps.clone(),
            &self.coordinator_worker_circuits,
            &self.proof_verifier.library,
        )
        .await
    }

    pub async fn get_newest_checkpoint(&self) -> anyhow::Result<QEDCheckpointSyncInfoCompact<F>> {
        let checkpoint_id = self.checkpoint_id().await?;
        self.store_reader
            .get_checkpoint_sync_info_compact(checkpoint_id)
            .await
    }
}
