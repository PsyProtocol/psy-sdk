use crate::config::RealmNodeConfig;
use crate::rpc::CheckpointSyncInfo;
use crate::{RealmInternalQueue, C, D, F};
use fred::prelude::KeysInterface;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq::traits::KVQSerializable;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_core::config::network_constants::QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL;
use qed_core::job::history_queue::CheckpointHistoryQueueConsumerAsyncImm;
use qed_core::job::id::{ProvingJobCircuitType, ProvingJobDataId};
use qed_core::job::worker_queue::WorkerEventTransmitterAsyncImm;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;
use qed_node::nimpl::new_fred_pool;
use qed_node::nimpl::proof_store_fred::{ProofStoreFred, PS_HISTORY_QUEUE_KEY_PREFIX};
use qed_node::realm::state::processor::{RealmConfig, RealmProcessorContext};
use qed_node_common::verifier::get_cached_generic_verifier;
use qed_store::traits::qdatastore::qtreedata::QEDComboDataStoreReaderWriterSync;
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
    pub local_checkpoint_id: u64,
}

pub async fn run_realm_processor(config: RealmNodeConfig) -> anyhow::Result<()> {
    let realm_processor = RealmProcessor::new(config).await?;
    let _ = realm_processor.start().await?;
    Ok(())
}

pub const REALM_PROCESSOR_SUFFIX: &str = "RP";

impl RealmProcessor {
    pub async fn new(config: RealmNodeConfig) -> anyhow::Result<Self> {
        info!("Realm Processor Config: {:?}", config);
        let pool =
            new_fred_pool(&config.redis.redis_uri, config.redis.pool_size.unwrap_or(20)).await?;
        let realm_qps = ProofStoreFred::new2(
            pool,
            config.queue.worker_queue_suffix,
            config.queue.notifications_queue_suffix,
            Some(config.queue.proof_store_key_suffix.as_str()),
            Some(config.queue.proof_store_key_suffix.as_str()),
        );
        let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore> =
            KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(KVQlibmdbxStore::new_write(
                &config.db.path,
            )?);

        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
        let realm_config = RealmConfig::get_standard(config.realm.node_id, config.realm.realm_id);
        let processor = RealmProcessor {
            realm_config,
            queue: realm_qps,
            store: store_reader,
            proof_verifier,
            local_checkpoint_id: 0,
        };
        Ok(processor)
    }

    pub async fn start(mut self) -> anyhow::Result<JoinHandle<()>> {
        info!("Realm Processor starting");
        let st = Arc::new(self.store.dup());
        st.initialize_store()?;
        let realm_qps = Arc::new(self.queue.clone());
        let mut context = RealmProcessorContext::new(
            self.realm_config,
            st.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            self.proof_verifier.clone(),
        ).await?;
        info!("Realm Processor started");
        loop {
            info!("Waiting for latest checkpoint");
            match self.sync_checkpoint(&mut context).await {
                Ok(false) | Err(_) => {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }
                _ => {}
            }
            info!("Start building block");
            let proving_data_job_id: ProvingJobDataId = match self.build_block(&mut context).await {
                Ok(job_id) => job_id,
                Err(err) => {
                    error!("Error building block: {:?}", err);
                    continue;
                }
            };
            info!("Pushing job id to queue: {:?}", proving_data_job_id);
            self.queue.produce_proof(proving_data_job_id).await?;
            // Send the job id to the channel for the next step
            // if let Err(err) = self.queue.cdq_push_imm(proving_data_job_id).await {
            //     error!("Error chq_push_imm: {:?}", err);
            // };
            info!("Pushing job to queueue done");
        }
    }

    pub async fn sync_checkpoint(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
    ) -> anyhow::Result<bool> {
        let block = self.wait_latest_checkpoint().await;
        match block {
            Ok(block) => {
                // checkpoint.l2_block_state
                let checkpoint_id = block.compact.l2_block_state.checkpoint_id;

                if self.local_checkpoint_id >= checkpoint_id {
                    info!("Local checkpoint is up to date");
                    return Ok(false);
                }

                if self.local_checkpoint_id >= block.lastest_checkpoint_id {
                    info!("Local checkpoint is latest");
                    return Ok(true);
                }

                info!(?checkpoint_id, "Checkpoint received");
                match context.handle_checkpoint_sync(block.compact.clone()).await {
                    Ok(_) => {
                        info!(?checkpoint_id, "Sync to new checkpoint");
                        info!("Checkpoint sync reg users: {:?}", block.compact.registered_users);
                        if  self.local_checkpoint_id + 1 == block.lastest_checkpoint_id && block.lastest_checkpoint_id == checkpoint_id {
                            info!("Local checkpoint is latest");
                            self.local_checkpoint_id = checkpoint_id;
                            return Ok(true);
                        }
                    
                        Ok(false)
                    }
                    Err(err) => {
                        error!(?checkpoint_id, ?err, "Error sync checkpoint");
                        Err(err)
                    }
                }
            }
            Err(err) => {
                error!(?self.local_checkpoint_id, "Error getting checkpoint sync info: {:?}", err);
                Err(err)
            }
        }
    }

    pub async fn build_block(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
    ) -> anyhow::Result<ProvingJobDataId> {
        context.build_block().await?;
        let realm_worker_output_job_id = self
            .queue
            .wait_for_block_proving_jobs_imm(self.local_checkpoint_id + 1)
            .await?;
        Ok(ProvingJobDataId::new(
            self.local_checkpoint_id + 1,
            realm_worker_output_job_id,
        ))
    }

    pub async fn wait_latest_checkpoint(
        &self,
    ) -> anyhow::Result<CheckpointSyncInfo> {
        self.queue.consume_checkpoint_async_info().await
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
