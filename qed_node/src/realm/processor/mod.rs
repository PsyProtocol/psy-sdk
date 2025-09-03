mod slot_phase;

use std::ops::Deref;
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
use qed_store::queue::QPendingUserStoreAsyncImm;
use qed_store::queue::task_queue::{QProvingTaskStore, QProvingTaskStoreImpl};
use qed_store::store::QEDStore;
use std::sync::{atomic, Arc};
use std::sync::atomic::AtomicBool;
use std::thread::sleep;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use anyhow::{anyhow, bail};
use futures::future::{err, ok};
use tower_http::follow_redirect::policy::PolicyExt;
use tracing::{debug, error, info, warn};
use qed_core::config::network_constants::COORDINATOR_USER_TREE_HEIGHT;
use qed_core::data::qhashout::QHashOut;
use qed_store::queue::new_redis_async_pool;
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::queue::ProofStoreRedisAsync;
use qed_store::store::journal::{Journal, JournalStore};
use crate::common::clock::SlotTimer;
use crate::common::slot::{Clock, LocalClock, Slot};
use crate::common::retry::Retryable;
use crate::realm::processor::slot_phase::SlotPhase;

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
    pub store: QEDStore,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub task_store: Arc<QProvingTaskStoreImpl>,
    pub slot_timer: SlotTimer<LocalClock>,
    pub remote_latest_slot: u64,
    pub is_synced: AtomicBool,
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
        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
        let realm_config = RealmConfig::get_standard(config.realm.node_id, config.realm.realm_id);
        // Use the same ProofStoreRedisAsync for checkpoint sync
        let sync_checkpoint = Arc::new(realm_qps.clone());
        let processor = RealmProcessor {
            realm_config,
            sync_proof: realm_qps,
            sync_checkpoint,
            store,
            proof_verifier,
            task_store: Arc::new(task_store),
            slot_timer: SlotTimer::new(LocalClock),
            remote_latest_slot: 0,
            is_synced: AtomicBool::new(false),
        };
        Ok(processor)
    }

    async fn context(
        &self,
    ) -> anyhow::Result<ConcreteRealmProcessorContext> {
        let realm_qps = Arc::new(self.sync_proof.clone());
        RealmProcessorContext::<
            JournalStore<QEDStore>,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
            QProvingTaskStoreImpl,
        >::new(
            self.realm_config,
            JournalStore::new(self.store.clone()),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            self.task_store.clone(),
            self.proof_verifier.clone(),
        ).await
    }

    pub async fn start(mut self) -> anyhow::Result<JoinHandle<()>> {
        info!("Realm Processor starting");
        let sync_queue = Arc::new(self.sync_proof.clone());
        // Check for incomplete consumption state on startup
        if let Ok(Some(last_state)) = sync_queue.get_last_peek_offset().await {
            info!("🔄 Found incomplete consumption state for checkpoint {} on startup", last_state.checkpoint_id);
        }

        if let Ok(local_latest_l2_block_state) = self.store.get_latest_l2_block_state().await {
            info!(
                "local_latest_l2_block_state: {:?}",
                local_latest_l2_block_state
            );

            let pending_users_count = sync_queue.get_pending_users_count().await?;
            info!("Found {} pending users in Redis queue during recovery", pending_users_count);
        }

        let slot_timer = self.slot_timer.clone();
        let mut context = self.context().await?;
        let mut pending_checkpoint_id = None;
        loop {
            tokio::select! {
                biased;
                checkpoint_sync_result = self.ensure_checkpoint_sync() => {
                    match checkpoint_sync_result {
                        Ok(ret) => {
                            debug!("Checkpoint sync completed");
                            if let Some(pending_checkpoint) = pending_checkpoint_id {
                                let realm_root = context.store.get_user_sub_tree_merkle_proof(
                                    pending_checkpoint,
                                    0,
                                    COORDINATOR_USER_TREE_HEIGHT,
                                    self.realm_config.realm_id as u64,
                                ).await?;
                                info!("pending checkpoint id: {}, latest checkpoint id: {}, realm_root: {}, expected realm_root: {}",
                                    pending_checkpoint, ret.latest_checkpoint_id, realm_root.value, ret.realm_root);

                                if ret.latest_checkpoint_id >= pending_checkpoint && realm_root.value == ret.realm_root {
                                    context.commit(pending_checkpoint).await?;
                                    pending_checkpoint_id = None;
                                    info!("Commit checkpoint {}", pending_checkpoint);
                                } else {
                                    if ret.latest_checkpoint_id > pending_checkpoint + 1 || ret.latest_checkpoint_id < pending_checkpoint - 1 {
                                        warn!("Invalid checkpoint sync result, rollback");
                                        context.rollback(pending_checkpoint).await?;
                                    } else {
                                        continue
                                    }
                                }
                            }
                        },
                        Err(err) => {
                            error!("Checkpoint sync failed: {:?}", err);
                        }
                    }
                },
                slot = slot_timer.wait_for_next_slot() => {
                    info!("Next slot: {}", slot);
                    if let Some(pending_checkpoint) = pending_checkpoint_id {
                        warn!("Pending checkpoint id: {}, continue", pending_checkpoint);
                        continue
                    }
                    if let Err(err) = self.validate_slot() {
                        warn!("Error validating slot: {:?}", err);
                        continue
                    }

                    if !self.is_synced.load(atomic::Ordering::Relaxed) {
                        warn!("Is syncing, continue");
                        continue;
                    }
                    // TODO remove the code
                    // if let SlotPhase::BuildPhase(build_phase_start) = SlotPhase::get_build_phase(self.slot_timer.deref()){
                    //     let current_timestamp = self.slot_timer.get_current_timestamp();
                    //     if current_timestamp < build_phase_start {
                    //         let tt = build_phase_start - current_timestamp;
                    //         info!("Waiting for build phase to start: sleep {} ms, slot: {}", tt, slot);
                    //         tokio::time::sleep(Duration::from_millis(tt)).await;
                    //     }
                    // }

                    info!("Start building block");
                    // Build block based on slot timing
                    pending_checkpoint_id = None;
                    let local_latest_checkpoint_id = self.get_local_latest_checkpoint_id().await?;
                    let next_checkpoint_id = local_latest_checkpoint_id + 1;
                    let has_tasks = context.has_pending_tasks(next_checkpoint_id).await?;
                    if !has_tasks {
                        debug!("No, pending tasks for checkpoint {}, skipping block construction", next_checkpoint_id);
                        continue;
                    }
                    // TODO async
                    let proving_data_job_id: ProvingJobDataId = match self.build_block(next_checkpoint_id, &mut context).await {
                        Ok(job_id) => job_id,
                        Err(err) => {
                            error!("Error building block: {:?}, slot: {}", err, slot);
                            continue;
                        }
                    };

                    // if next_checkpoint_id <= self.get_local_latest_checkpoint_id().await? {
                    //     warn!("Local latest checkpoint id is greater than pending checkpoint id, skipping block construction");
                    //     context.rollback(next_checkpoint_id).await?;
                    //     continue
                    // }

                    // if !self.slot_timer.is_can_reach_to_next_slot() {
                    //     warn!("Is not reach to next slot, skipping block construction");
                    //     context.rollback(next_checkpoint_id).await?;
                    //     continue;
                    // }
                    debug!("Pushing job id to queue: {:?}, slot: {}", proving_data_job_id, slot);
                    self.sync_proof.chq_push_imm(proving_data_job_id).await?;
                    pending_checkpoint_id = Some(next_checkpoint_id);
                    info!("Pushing job to queueue done");
                }
            }
        }
    }

    async fn ensure_checkpoint_sync(
        &mut self,
    ) -> anyhow::Result<SyncCheckpointResult> {
        let mut context = self.context().await?;
        loop {
            match self.sync_checkpoint(&mut context).await {
                Ok(ret) => {
                    self.is_synced.store(ret.is_synced, atomic::Ordering::Relaxed);
                    if ret.is_synced {
                        // Sync completed
                        return Ok(ret)
                    }
                },
                Err(err) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    bail!("Checkpoint sync attempt failed: {:?}", err)
                }
            }
        }
    }

    pub async fn sync_checkpoint(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
    ) -> anyhow::Result<SyncCheckpointResult> {
        let (expected_checkpoint,local_checkpoint_id) = if let Ok(local_checkpoint_id) = self.get_local_latest_checkpoint_id().await {
            // Get the next expected checkpoint
            (local_checkpoint_id + 1, local_checkpoint_id)
        } else {
            (0, 0)
        };
        debug!("local_checkpoint_id {}, expected_checkpoint {}",local_checkpoint_id, expected_checkpoint);

        // Wait for the next checkpoint sync info
        let block = self.sync_checkpoint.wait_for_next_item_imm::<CheckpointSyncInfo<F>>(
            qed_core::config::network_constants::QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL,
            expected_checkpoint
        ).await;

        match block {
            Ok(block) => {
                // checkpoint.l2_block_state
                let checkpoint_id = block.compact.l2_block_state.checkpoint_id;
                let mut ret = SyncCheckpointResult{
                    checkpoint_id,
                    latest_checkpoint_id: block.latest_checkpoint_id,
                    slot: block.compact.slot,
                    is_synced: false,
                    realm_root: block.realm_root,
                };
                info!("Checkpoint received checkpoint_id: {},latest_checkpoint_id: {} ,local_checkpoint_id: {}", checkpoint_id, block.latest_checkpoint_id, local_checkpoint_id);
                if local_checkpoint_id >= block.latest_checkpoint_id && local_checkpoint_id > 0 {
                    info!("Local checkpoint is latest");
                    self.remote_latest_slot = block.compact.slot;
                    ret.is_synced = true;
                    return Ok(ret);
                }
                if local_checkpoint_id >= checkpoint_id && local_checkpoint_id > 0 {
                    info!("Local checkpoint is up to date");
                    return Ok(ret);
                }

                info!("Syncing checkpoint");
                match context.handle_checkpoint_sync(block.compact.clone()).await {
                    Ok(_) => {
                        info!(?checkpoint_id, "Sync to new checkpoint");
                        info!("Checkpoint sync reg users len: {}", block.compact.registered_users.len());
                        context.commit(checkpoint_id).await?;

                        // Check updated pending users count
                        let pending_users_count = context.sync_queue.get_pending_users_count().await?;
                        info!("Pending users count after checkpoint sync: {}", pending_users_count);

                        if local_checkpoint_id + 1 == block.latest_checkpoint_id && block.latest_checkpoint_id == checkpoint_id
                            ||  local_checkpoint_id == checkpoint_id && block.latest_checkpoint_id == checkpoint_id && local_checkpoint_id == 0
                        {
                            info!("Local checkpoint is latest");
                            self.remote_latest_slot = block.compact.slot;
                            ret.is_synced = true;
                            return Ok(ret);
                        }
                        Ok(ret)
                    }
                    Err(err) => {
                        error!(?checkpoint_id, ?err, "Error sync checkpoint");
                        context.rollback(checkpoint_id).await?; //todo
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
        &self,
        next_checkpoint_id: u64,
        context: &ConcreteRealmProcessorContext,
    ) -> anyhow::Result<ProvingJobDataId> {
        self.retry_with_backoff(&format!("build block for checkpoint {}", next_checkpoint_id), || async {
            // Build block with enhanced error handling(all logic including logging is inside context.build_block)
            match context.build_block().await {
                Ok(job_id) => {
                    // Success - consumption is already committed in build_block
                    //context.commit_offset().await?;
                    Ok(ProvingJobDataId::new(next_checkpoint_id, job_id))
                },
                Err(err) => {
                    // Rollback database changes
                    context.rollback(next_checkpoint_id).await?;

                    error!("Build block failed for checkpoint {}: {:?}", next_checkpoint_id, err);
                    Err(err)
                }
            }
        }).await
    }

    fn validate_slot(&self) -> anyhow::Result<()> {
        let slot = self.slot_timer.get_current_slot();
        if !self.is_current_slot() {
            bail!("Not in current slot, slot: {}, remote latest slot: {}", slot, self.remote_latest_slot)
        }

        // if !self.slot_timer.is_can_reach_to_next_slot() {
        //     bail!("Not reach to next slot")
        // }
        Ok(())
    }

    fn is_current_slot(&self) -> bool {
        self.remote_latest_slot == 0 || self.slot_timer.get_current_slot() >= self.remote_latest_slot
    }
    pub async fn get_local_latest_checkpoint_id(&self) -> anyhow::Result<u64> {
        let state = self
            .store
            .get_latest_l2_block_state()
            .await
            .map_err(|err| anyhow!("Error getting latest l2 block state: {:?}", err))?;
        Ok(state.checkpoint_id)
    }
}

impl Retryable for RealmProcessor {}

pub struct SyncCheckpointResult {
    pub checkpoint_id: u64,
    pub latest_checkpoint_id: u64,
    pub slot: u64,
    pub is_synced: bool,
    pub realm_root: QHashOut<F>,
}