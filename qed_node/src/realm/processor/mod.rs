use crate::common::verifier::get_cached_generic_verifier;
use crate::realm::config::RealmNodeConfig;
use crate::realm::state::processor::{RealmConfig, RealmProcessorContext};
use crate::realm::{C, D, F};
use anyhow::anyhow;
use qed_core::job::history_queue::{
    CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm,
};
use qed_core::job::id::ProvingJobDataId;
use qed_core::job::worker_queue::WorkerEventTransmitterAsyncImm;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::queue::new_redis_async_pool;
use qed_store::queue::task_queue::{QProvingTaskStore, QProvingTaskStoreImpl};
use qed_store::queue::ProofStoreRedisAsync;
use qed_store::store::journal::{Journal, JournalStore};
use qed_store::store::QEDStore;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

type ConcreteRealmProcessorContext = RealmProcessorContext<
    JournalStore<QEDStore>,
    ProofStoreRedisAsync,
    ProofStoreRedisAsync,
    ProofStoreRedisAsync,
    ProofStoreRedisAsync,
    QProvingTaskStoreImpl,
>;

pub struct RealmProcessor {
    pub realm_config: RealmConfig,
    pub sync_proof: ProofStoreRedisAsync,
    pub sync_checkpoint: Arc<ProofStoreRedisAsync>,
    pub store: Arc<JournalStore<QEDStore>>,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub task_store: Arc<QProvingTaskStoreImpl>,
}

pub async fn run_realm_processor(config: RealmNodeConfig) -> anyhow::Result<()> {
    let realm_processor = RealmProcessor::new(config).await?;
    let _ = realm_processor.start().await?;
    Ok(())
}

impl RealmProcessor {
    pub async fn new(config: RealmNodeConfig) -> anyhow::Result<Self> {
        info!("Realm Processor Config: {:?}", config);
        let pool = new_redis_async_pool(
            config.redis.redis_uri.as_str(),
            config.redis.pool_size.unwrap_or(10)
        ).await?;
        let task_store = QProvingTaskStoreImpl::new(
            &config.redis.redis_uri.as_str(),
            config.redis.pool_size.unwrap_or(10),
        )
        .await?;
        let realm_qps = ProofStoreRedisAsync::new(
            pool,
            config.queue.queue_biz_key,
        ).await?;
        let store = QEDStore::new(&config.backend.to_backend()).await?;
        let store = Arc::new(JournalStore::new(store));
        let store_reader = store.clone();

        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
        let realm_config = RealmConfig::get_standard(config.realm.node_id, config.realm.realm_id);
        // Use the same ProofStoreRedisAsync for checkpoint sync
        let sync_checkpoint = Arc::new(realm_qps.clone());
        let processor = RealmProcessor {
            realm_config,
            sync_proof: realm_qps,
            sync_checkpoint,
            store: store_reader,
            proof_verifier,
            task_store: Arc::new(task_store),
        };
        Ok(processor)
    }

    pub async fn start(mut self) -> anyhow::Result<JoinHandle<()>> {
        info!("Realm Processor starting");
        let st = self.store.clone();
        let realm_qps = Arc::new(self.sync_proof.clone());
        let mut context: ConcreteRealmProcessorContext = RealmProcessorContext::<
            JournalStore<QEDStore>,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
            QProvingTaskStoreImpl,
        >::new(
            self.realm_config,
            st.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            self.task_store.clone(),
            self.proof_verifier.clone(),
        )
        .await?;
        info!("Realm Processor started");
        loop {
            info!("Waiting for latest checkpoint");
            match self.sync_checkpoint(&mut context).await {
                Ok(false) => {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }
                Err(err) => {
                    warn!("Error syncing checkpoint: {:?}", err);
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }

                _ => {}
            }
            info!("Start building block");
            let proving_data_job_id: ProvingJobDataId =
                match self.build_block(&mut context, &realm_qps).await {
                    Ok(job_id) => job_id,
                    Err(err) => {
                        error!("Error building block: {:?}", err);
                        continue;
                    }
                };
            info!("Pushing job id to queue: {:?}", proving_data_job_id);
            self.sync_proof.chq_push_imm(proving_data_job_id).await?;
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
        let (expected_checkpoint, local_checkpoint_id) =
            if let Ok(local_checkpoint_id) = self.get_local_latest_l2_block_state().await {
                // Get the next expected checkpoint
                (local_checkpoint_id + 1, local_checkpoint_id)
            } else {
                (0, 0)
            };
        debug!(
            "local_checkpoint_id {}, expected_checkpoint {}",
            local_checkpoint_id, expected_checkpoint
        );

        // Wait for the next checkpoint sync info
        let block = self.sync_checkpoint.wait_for_next_item_imm::<CheckpointSyncInfo<F>>(
            qed_core::config::network_constants::QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL,
            expected_checkpoint
        ).await;

        match block {
            Ok(block) => {
                // checkpoint.l2_block_state
                let checkpoint_id = block.compact.l2_block_state.checkpoint_id;

                info!(
                    "Checkpoint received checkpoint_id: {}, local_checkpoint_id: {}",
                    checkpoint_id, local_checkpoint_id
                );
                if local_checkpoint_id >= checkpoint_id && local_checkpoint_id > 0 {
                    info!("Local checkpoint is up to date");
                    return Ok(false);
                }

                if local_checkpoint_id >= block.latest_checkpoint_id && local_checkpoint_id > 0 {
                    info!("Local checkpoint is latest");
                    return Ok(true);
                }

                info!("Syncing checkpoint");
                match context.handle_checkpoint_sync(block.compact.clone()).await {
                    Ok(_) => {
                        info!(?checkpoint_id, "Sync to new checkpoint");
                        info!(
                            "Checkpoint sync reg users: {:?}",
                            block.compact.registered_users
                        );
                        if local_checkpoint_id + 1 == block.latest_checkpoint_id
                            && block.latest_checkpoint_id == checkpoint_id
                            || local_checkpoint_id == checkpoint_id
                                && block.latest_checkpoint_id == checkpoint_id
                                && local_checkpoint_id == 0
                        {
                            info!("Local checkpoint is latest");
                            return Ok(true);
                        }
                        self.store.commit(checkpoint_id)?;
                        Ok(false)
                    }
                    Err(err) => {
                        error!(?checkpoint_id, ?err, "Error sync checkpoint");
                        self.store.rollback(checkpoint_id)?;
                        Err(err)
                    }
                }
            }
            Err(err) => {
                error!(
                    ?local_checkpoint_id,
                    "Error getting checkpoint sync info: {:?}", err
                );
                Err(err)
            }
        }
    }

    pub async fn build_block(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
        realm_qps: &ProofStoreRedisAsync,
    ) -> anyhow::Result<ProvingJobDataId> {
        let local_latest_checkpoint_id = self.get_local_latest_l2_block_state().await?;
        let next_checkpoint_id = local_latest_checkpoint_id + 1;
        self.store.commit(local_latest_checkpoint_id)?;

        match context.build_block().await {
            Ok(_) => {
                let realm_worker_output_job_id = self
                    .sync_proof
                    .wait_for_block_proving_jobs_imm(next_checkpoint_id)
                    .await?;
                Ok(ProvingJobDataId::new(
                    next_checkpoint_id,
                    realm_worker_output_job_id,
                ))
            }
            Err(err) => {
                self.store.rollback(next_checkpoint_id)?;
                Err(err)
            }
        }
    }

    pub async fn get_local_latest_l2_block_state(&self) -> anyhow::Result<u64> {
        let state = self
            .store
            .get_latest_l2_block_state()
            .await
            .map_err(|err| anyhow!("Error getting latest l2 block state: {:?}", err))?;
        Ok(state.checkpoint_id)
    }
}
