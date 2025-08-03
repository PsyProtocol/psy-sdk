use crate::common::verifier::get_cached_generic_verifier;
use crate::realm::config::RealmNodeConfig;
use crate::realm::state::processor::{RealmConfig, RealmProcessorContext};
use crate::realm::{C, D, F};
use qed_core::job::history_queue::{
    CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm,
};
use qed_core::job::id::ProvingJobDataId;
use qed_core::job::worker_queue::WorkerEventTransmitterAsyncImm;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_data::models::checkpoint::sync_info::CheckpointError;
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::queue::new_redis_async_pool;
use qed_store::queue::task_queue::{JobTaskStore, JobTaskStoreImpl};
use qed_store::queue::ProofStoreRedisAsync;
use qed_store::store::journal::{Journal, JournalStore};
use qed_store::store::QEDStore;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

type ConcreteRealmProcessorContext = RealmProcessorContext<
    JournalStore<QEDStore>,
    ProofStoreRedisAsync,
    ProofStoreRedisAsync,
    ProofStoreRedisAsync,
    ProofStoreRedisAsync,
>;

pub struct RealmProcessor {
    pub realm_config: RealmConfig,
    pub sync_proof: ProofStoreRedisAsync,
    pub sync_checkpoint: Arc<ProofStoreRedisAsync>,
    pub store: Arc<JournalStore<QEDStore>>,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub job_task_store: Arc<JobTaskStoreImpl>,
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
            config.redis.pool_size.unwrap_or(10),
        )
        .await?;
        let task_store = JobTaskStoreImpl::new(
            &config.redis.redis_uri.as_str(),
            config.redis.pool_size.unwrap_or(10),
        )
        .await?;
        let realm_qps = ProofStoreRedisAsync::new2(
            pool,
            &config.queue.worker_queue_suffix,
            &config.queue.notifications_queue_suffix,
            &config.queue.proof_store_key_suffix,
            &config.queue.proof_store_key_suffix,
        )
        .await?;
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
            job_task_store: Arc::new(task_store),
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
        >::new(
            self.realm_config,
            st.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
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
        let block = self.wait_latest_checkpoint().await;
        let local_checkpoint_id = self.get_local_latest_l2_block_state().await;
        match block {
            Ok(block) => {
                // checkpoint.l2_block_state
                let checkpoint_id = block.compact.l2_block_state.checkpoint_id;

                info!(
                    "Checkpoint received checkpoint_id: {}, local_checkpoint_id: {}",
                    checkpoint_id, local_checkpoint_id
                );
                if local_checkpoint_id >= checkpoint_id {
                    info!("Local checkpoint is up to date");
                    return Ok(false);
                }

                if local_checkpoint_id >= block.latest_checkpoint_id {
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
                        {
                            info!("Local checkpoint is latest");
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
        let local_latest_checkpoint_id = self.get_local_latest_l2_block_state().await;
        let next_checkpoint_id = local_latest_checkpoint_id + 1;
        self.store.commit(local_latest_checkpoint_id)?;
        if let Err(err) = context.build_block().await {
            self.store.rollback(next_checkpoint_id)?;
            return Err(err);
        }
        {
            let mut task_graph = context.proof_store.task_graph.lock().await;
            let sorted_tasks = task_graph.ts_task();
            self.job_task_store.save_task_topology(sorted_tasks).await?;
            task_graph.clear();
        }
        let realm_worker_output_job_id = self
            .sync_proof
            .wait_for_block_proving_jobs_imm(next_checkpoint_id)
            .await?;

        Ok(ProvingJobDataId::new(
            next_checkpoint_id,
            realm_worker_output_job_id,
        ))
    }

    pub async fn wait_latest_checkpoint(&self) -> anyhow::Result<CheckpointSyncInfo<F>> {
        // Get the next expected checkpoint
        let current_checkpoint = self.get_local_latest_l2_block_state().await;
        let expected_checkpoint = current_checkpoint + 1;

        // Wait for the next checkpoint sync info
        self.sync_checkpoint.wait_for_next_item_imm::<CheckpointSyncInfo<F>>(
            qed_core::config::network_constants::QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL,
            expected_checkpoint
        ).await
    }

    pub async fn get_local_latest_l2_block_state(&self) -> u64 {
        match self.store.get_latest_l2_block_state().await {
            Ok(state) => state.checkpoint_id,
            Err(e) => {
                match e.downcast::<CheckpointError>() {
                    Ok(CheckpointError::NotFound) => {
                        warn!("Latest L2 block state not found, setting current local checkpoint ID to 0");
                    }
                    Ok(CheckpointError::Other(e)) | Err(e) => {
                        error!("Failed to get latest L2 block state: {:?}", e);
                    }
                }
                0
            }
        }
    }
}
