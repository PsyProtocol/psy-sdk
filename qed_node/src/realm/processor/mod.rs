pub mod processor_v2;
use plonky2::plonk::proof::ProofWithPublicInputs;
pub use processor_v2::*;
use qed_data::guta::api::{GUTARealmCheckpointResult, SubmitGUTARealmResultAPINoProofInput};

use std::ops::Deref;
use crate::common::verifier::get_cached_generic_verifier;
use crate::common_v2::traits::realm::CoordinatorClient;
use crate::coordinator::client_v2::ConcreteCoordinatorClient;
use crate::realm::config::RealmNodeConfig;
use crate::realm::state::processor::{RealmConfig, RealmProcessorContext};
use crate::realm::{edge, C, D, F};
use qed_core::config::network_constants::{COORDINATOR_USER_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT};
use qed_core::job::history_queue::{
    CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm,
};
use qed_core::job::id::ProvingJobDataId;
use qed_core::job::worker_queue::WorkerEventTransmitterAsyncImm;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_crypto::hash::merkle::utils::common::{QMerkleNode, SimpleMerkleNodeKey};
use qed_crypto::hash::traits::hasher::MerkleZeroHasher;
use qed_data::config::genesis_config::GenesisConfig;
use qed_store::queue::QPendingUserStoreAsyncImm;
use qed_store::queue::task_queue::{QProvingTaskStore, QProvingTaskStoreImpl};
use qed_store::store::QEDStore;
use std::sync::{atomic, Arc};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::thread::sleep;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use anyhow::{anyhow, bail};
use futures::future::{err, ok};
use tower_http::follow_redirect::policy::PolicyExt;
use tracing::{debug, error, info, trace, warn};
use qed_core::data::qhashout::QHashOut;
use qed_store::queue::new_redis_async_pool;
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::queue::ProofStoreRedisAsync;
use qed_store::store::journal::{Journal, JournalStore};
use crate::common::clock::SlotTimer;
use crate::common::slot::{Clock, LocalClock, Parity, Slot, SLOT_SIZE};
use crate::common::retry::Retryable;
use qed_core::config::network_constants::{REALM_USER_TREE_HEIGHT, USERS_PER_REALM};
use qed_data::config::store_config::QEDHasher;
use qed_core::config::network_constants::{MAX_CONTRACT_STATE_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT};
use qed_data::qdata::user::QEDUserLeaf;
use qed_crypto::hash::traits::qhashable::QFieldHashable;
use qed_store::node::realm::QEDRealmStoreWriterAsyncImm;
use qed_crypto::common::user_id::get_user_id_from_registration_id;
use plonky2::field::types::Field;
use qed_data::traits::qdatastore::{qtreedata::QTreeDataStoreWriterSync, qmetadata::QMetaDataStoreWriterSync};
use std::{str::FromStr, collections::HashMap};
use plonky2::field::goldilocks_field::GoldilocksField;
use super::backup::{RealmS3BackupClient, try_backup_realm_checkpoint};
use tokio::sync::mpsc;

struct RealmBackupRequest {
    checkpoint_id: u64,
    pair_to_set: Vec<(Vec<u8>, Vec<u8>)>,
    removed_keys: Vec<Vec<u8>>,
}
use crate::realm::state::edge_queue_helper::RealmEdgeQueueHelper;
use crate::realm::state::queue_factory::QueueFactory;

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
    pub max_processed_end_caps_per_block: Option<isize>,
    pub sync_proof: ProofStoreRedisAsync,
    pub sync_checkpoint: Arc<ProofStoreRedisAsync>,
    pub store: QEDStore,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub task_store: Arc<QProvingTaskStoreImpl>,
    pub slot_timer: SlotTimer<LocalClock>,
    pub remote_latest_slot: AtomicU64,
    pub config_path: String,
    pub is_synced: AtomicBool,
    pub pending_checkpoint_id: AtomicU64,
    pub shutdown_requested: Arc<AtomicBool>,
    pub backup_tx: Option<mpsc::UnboundedSender<RealmBackupRequest>>,
    pub queue_helper: Arc<RealmEdgeQueueHelper<F>>,
    pub coordinator_client: Arc<ConcreteCoordinatorClient>,
}

pub async fn run_realm_processor(config: RealmNodeConfig, shutdown_requested: Arc<AtomicBool>) -> anyhow::Result<()> {
    let mut realm_processor = RealmProcessor::new(config, shutdown_requested).await?;
    let _ = realm_processor.start().await?;
    Ok(())
}

impl RealmProcessor {
    pub async fn new(config: RealmNodeConfig, shutdown_requested: Arc<AtomicBool>) -> anyhow::Result<Self> {
        info!("Realm Processor Config: {:?}", config);
        let task_store = QProvingTaskStoreImpl::new(
            &config.redis.redis_uri.as_str(),
            config.redis.pool_size.unwrap_or(10),
        )
        .await?;
        let realm_qps = ProofStoreRedisAsync::new(
            &config.redis.redis_uri,
            config.queue.queue_biz_key,
        ).await?;
        let store = QEDStore::new(&config.backend.to_backend()).await?;
        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
        let realm_config = RealmConfig::get_standard(config.realm.realm_id);
        let sync_checkpoint = Arc::new(realm_qps.clone());

        // Initialize backup client
        let backup_tx = match RealmS3BackupClient::new_from_env(config.realm.realm_id).await {
            Ok(client) => {
                info!("✅ S3 backup client initialized");
                let (tx, rx) = mpsc::unbounded_channel();
                tokio::spawn(async move {
                    RealmProcessor::backup_task(rx, client, config.realm.realm_id).await;
                });
                info!("Started realm backup task");
                Some(tx)
            },
            Err(e) => {
                warn!("⚠️ S3 backup client initialization failed: {}", e);
                None
            }
        };

        let coordinator_client = Arc::new(ConcreteCoordinatorClient::new(config.coordinator_addr.clone())?);
        let queue_helper = QueueFactory::create_rsmq_helper::<F>(
            &config.redis.redis_uri,
            config.redis.pool_size.unwrap_or(10),
            config.realm.realm_id,
            Arc::new(store.clone()),
        ).await?;

        edge::spawn_active_checkpoint_sync_task(
            config.realm.realm_id,
            Arc::new(store.clone()),
            sync_checkpoint.clone(),
            config.coordinator_addr.clone(),
        ).await?;

        let processor = RealmProcessor {
            realm_config,
            max_processed_end_caps_per_block: config.realm.max_processed_end_caps_per_block.clone(),
            sync_proof: realm_qps,
            sync_checkpoint,
            store,
            proof_verifier,
            task_store: Arc::new(task_store),
            slot_timer: SlotTimer::new(LocalClock),
            remote_latest_slot: AtomicU64::new(0),
            config_path: config.config_path.clone(),
            is_synced: AtomicBool::new(false),
            pending_checkpoint_id: AtomicU64::new(0),
            shutdown_requested,
            backup_tx,
            queue_helper: Arc::new(queue_helper),
            coordinator_client,
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
            self.max_processed_end_caps_per_block,
            JournalStore::new(self.store.clone()),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            self.task_store.clone(),
            self.proof_verifier.clone(),
        ).await
    }

    pub async fn start(mut self) -> anyhow::Result<()> {
        info!("Realm Processor starting");
        let sync_queue = Arc::new(self.sync_proof.clone());
        // Check for incomplete consumption state on startup
        if let Ok(Some(last_state)) = sync_queue.get_last_peek_offset().await {
            info!("🔄 Found incomplete consumption state for checkpoint {} on startup", last_state.checkpoint_id);
        }

        let build_ctx = self.context().await?;
        if let Ok(local_latest_l2_block_state) = self.store.get_latest_l2_block_state().await {
            info!(
                "local_latest_l2_block_state: {:?}",
                local_latest_l2_block_state.clone()
            );
            let pending_checkpoint_id = local_latest_l2_block_state.checkpoint_id + 1;
            if let Some(snapshot) = build_ctx.store.get_snapshot(pending_checkpoint_id)?{
                build_ctx.store.restore_snapshot(snapshot)?;
                self.pending_checkpoint_id.store(pending_checkpoint_id, Ordering::Relaxed);
                let block = self.sync_checkpoint(&self.context().await?, pending_checkpoint_id,local_latest_l2_block_state.checkpoint_id).await?;
                self.confirm_pending_checkpoint(&build_ctx, block).await?;
            }

            let pending_users_count = sync_queue.get_pending_users_count().await?;
            info!("Found {} pending users in Redis queue during recovery", pending_users_count);
        }
        tokio::join!(
            async {loop {
                if self.shutdown_requested.load(Ordering::Relaxed) && self.pending_checkpoint_id.load(Ordering::Relaxed) == 0 {
                    info!("Shutdown requested, exiting");
                    break;
                }
                if let Err(err) = self.sync_handle(&build_ctx).await {
                    error!("Sync handle error: {:?}", err);
                }
            }},
            async {loop {
                if self.shutdown_requested.load(Ordering::Relaxed) {
                    info!("Shutdown requested, exiting");
                    break;
                }
                if let Err(err) =  self.block_handle(&build_ctx).await {
                    let checkpoint = self.pending_checkpoint_id.load(Ordering::Relaxed);
                    error!("Rollback: block handle error: {:?}, pending_checkpoint_id: {}", err, checkpoint);
                    let _ = build_ctx.rollback(checkpoint).await;
                }
            }}
        );
        Ok(())
    }

    async fn sync_handle(&self, build_ctx: &ConcreteRealmProcessorContext) -> anyhow::Result<()> {
        let ret = self.ensure_checkpoint_sync(build_ctx).await?;
        trace!("Checkpoint sync completed");
        Ok(())
    }

    async fn confirm_pending_checkpoint(&self, build_ctx: &ConcreteRealmProcessorContext, ret: SyncCheckpointResult) -> anyhow::Result<()> {
        let checkpoint = self.pending_checkpoint_id.load(Ordering::Relaxed);
        if checkpoint > 0 {
            let realm_root = build_ctx.store.get_user_sub_tree_merkle_proof(
                checkpoint,
                0,
                COORDINATOR_USER_TREE_HEIGHT,
                self.realm_config.realm_id as u64,
            ).await?;
            trace!("pending checkpoint id: {}, synced checkpoint id: {}, latest checkpoint id: {}, local realm root: {}, remote realm root: {}",
                checkpoint, ret.checkpoint_id, ret.latest_checkpoint_id, realm_root.value, ret.realm_root);

            if ret.checkpoint_id >= checkpoint && realm_root.value == ret.realm_root {
                let (pair_to_set, remove_keys) = build_ctx.commit(ret.checkpoint_id).await?;
                self.pending_checkpoint_id.store(0, Ordering::Relaxed);
                info!("Commit checkpoint {}, latest_checkpoint_id: {}", checkpoint, ret.latest_checkpoint_id);

                // Auto backup after successful commit
                if let Some(backup_tx) = &self.backup_tx {
                    let pair_to_set = pair_to_set.into_iter().map(|pair| (pair.key, pair.value)).collect();
                    let request = RealmBackupRequest {
                        checkpoint_id: checkpoint,
                        pair_to_set,
                        removed_keys: remove_keys,
                    };
                    if let Err(e) = backup_tx.send(request) {
                        error!("❌ Failed to send realm backup request for checkpoint {}: {}", checkpoint, e);
                    }
                }
            }
            if ret.latest_checkpoint_id < checkpoint || ret.checkpoint_id > checkpoint + 2 {
                warn!("Rollback: invalid checkpoint sync result, latest_checkpoint_id: {}, pending checkpoint id: {}, synced checkpoint id: {}", ret.latest_checkpoint_id, checkpoint, ret.checkpoint_id);
                build_ctx.rollback(checkpoint).await?;
                self.pending_checkpoint_id.store(0, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    async fn block_handle(&self, build_ctx: &ConcreteRealmProcessorContext) -> anyhow::Result<()> {
        let slot = self.slot_timer.wait_for_next_slot().await;
        if slot.is_even() {
            return Ok(());
        }

        trace!("Next slot: {}", slot);
        let local_latest_checkpoint_id = self.get_local_latest_checkpoint_id().await?;
        if self.pending_checkpoint_id.load(Ordering::Relaxed) > 0 {
            warn!("Pending checkpoint id: {}, local_latest_checkpoint_id: {}, continue", self.pending_checkpoint_id.load(Ordering::Relaxed), local_latest_checkpoint_id);
            return Ok(());
        }
        if let Err(err) = self.validate_slot() {
            warn!("Error validating slot: {:?}", err);
            return Ok(());
        }

        if !self.is_synced.load(Ordering::Relaxed) {
            warn!("Is syncing, continue");
            return Ok(());
        }

        if !build_ctx.store.is_committed() {
            warn!("Store is not committed, continue, pending_checkpoint_id: {}", self.pending_checkpoint_id.load(Ordering::Relaxed));
            return Ok(());
        }

        // Build block based on slot timing
        self.pending_checkpoint_id.store(0, Ordering::Relaxed);
        let next_checkpoint_id = local_latest_checkpoint_id + 1;
        let has_tasks = build_ctx.has_pending_guta_tasks(next_checkpoint_id).await? || build_ctx.has_pending_user_tasks().await?;
        if !has_tasks {
            trace!("No pending tasks for checkpoint {}, skipping block construction", next_checkpoint_id);
            return Ok(());
        }
        let now = Instant::now();
        info!("Start building block checkpoint: {}, slot: {}", next_checkpoint_id, slot);
        match self.build_block(build_ctx, next_checkpoint_id).await {
            Ok(job_id) => {
                // self.sync_proof.chq_push_imm(job_id).await?;
                // submit guta task to coordinator
                self.submit_guta(build_ctx, job_id).await?;
                self.pending_checkpoint_id.store(next_checkpoint_id, Ordering::Relaxed);
                build_ctx.store.save_snapshot(next_checkpoint_id)?;
                info!("build complete checkpoint: {}, slot: {}, cost time: {:?}", next_checkpoint_id, slot, now.elapsed());
            }
            Err(err) => {
                // Rollback database changes
                build_ctx.rollback(next_checkpoint_id).await?;
                error!("Rollback: build block failed for checkpoint {}: {:?}", next_checkpoint_id, err);
            }
        }
        Ok(())
    }

    async fn submit_guta(
        &self,
        build_ctx: &ConcreteRealmProcessorContext,
        job_id: ProvingJobDataId,
    ) -> anyhow::Result<()> {
        use qed_core::job::traits::QProofStoreReaderAsync;
        let bytes = build_ctx.proof_store.get_bytes_by_id(job_id.job_id).await?;
        // Deserialize realm result
        let realm_result: GUTARealmCheckpointResult<F> = bincode::deserialize(&bytes)?;
        // Get proof with retry
        let proof: ProofWithPublicInputs<F, C, D> =  build_ctx.proof_store.get_proof_by_id(realm_result.proof_id.get_output_id()).await?;
        // let proof = self.get_proof_with_retry(proof_store, realm_result.proof_id.get_output_id()).await?;
        let input = SubmitGUTARealmResultAPINoProofInput::<F> {
            realm_id: self.realm_config.realm_id as u64,
            checkpoint_id: realm_result.checkpoint_id,
            guta_stats: realm_result.guta_stats,
            top_line_proof: realm_result.top_line_proof,
            checkpoint_tree_root: realm_result.checkpoint_tree_root,
            circuit_type: realm_result.proof_id.circuit_type,
        };
        self.coordinator_client.submit_guta_v1(&input, &bincode::serialize(&proof)?, input.realm_id).await
    }

    async fn ensure_checkpoint_sync(
        &self,
        build_ctx: &ConcreteRealmProcessorContext
    ) -> anyhow::Result<SyncCheckpointResult> {
        loop {
            let (expected_checkpoint,local_checkpoint_id) = if let Ok(local_checkpoint_id) = self.get_local_latest_checkpoint_id().await {
                // Get the next expected checkpoint
                (local_checkpoint_id + 1, local_checkpoint_id)
            } else {
                (0, 0)
            };
            match self.sync_checkpoint(&self.context().await?, expected_checkpoint, local_checkpoint_id).await {
                Ok(ret) => {
                    self.is_synced.store(ret.is_synced, atomic::Ordering::Relaxed);
                    self.confirm_pending_checkpoint(build_ctx, ret.clone()).await?;
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
    pub async fn sync_wait_expected_checkpoint(&self, expected_checkpoint: u64) -> anyhow::Result<CheckpointSyncInfo<F>>{
        // Wait for the next checkpoint sync info
         self.sync_checkpoint.wait_for_next_item_imm::<CheckpointSyncInfo<F>>(
            qed_core::config::network_constants::QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL,
            expected_checkpoint
        ).await
    }
    pub async fn sync_checkpoint(
        &self,
        sync_ctx: &ConcreteRealmProcessorContext,
        expected_checkpoint: u64,
        local_checkpoint_id: u64,
    ) -> anyhow::Result<SyncCheckpointResult> {
        trace!("local_checkpoint_id {}, expected_checkpoint {}",local_checkpoint_id, expected_checkpoint);
        let block = self.sync_wait_expected_checkpoint(expected_checkpoint).await;
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
                    self.remote_latest_slot.store(block.compact.slot, Ordering::Relaxed);
                    ret.is_synced = true;
                    return Ok(ret);
                }
                if local_checkpoint_id >= checkpoint_id && local_checkpoint_id > 0 {
                    info!("Local checkpoint is up to date");
                    return Ok(ret);
                }
                match sync_ctx.handle_checkpoint_sync(block.compact.clone()).await {
                    Ok(_) => {
                        info!("Checkpoint {} sync reg users len: {}", checkpoint_id, block.compact.registered_users.len());

                        if checkpoint_id == 0 {
                            self.initialize_genesis_state().await?;
                        }

                        let (pair_to_set, remove_keys) = sync_ctx.store.commit(None)?;
                        // Auto backup after successful commit
                        if let Some(backup_tx) = &self.backup_tx {
                            let pair_to_set = pair_to_set.into_iter().map(|pair| (pair.key, pair.value)).collect();
                            let request = RealmBackupRequest {
                                checkpoint_id,
                                pair_to_set,
                                removed_keys: remove_keys,
                            };
                            if let Err(e) = backup_tx.send(request) {
                                error!("❌ Failed to send realm backup request for checkpoint {}: {}", checkpoint_id, e);
                            }
                        }

                        let pending_users_count = sync_ctx.sync_queue.get_pending_users_count().await?;
                        trace!("Pending users count after checkpoint sync: {}", pending_users_count);

                        if local_checkpoint_id + 1 == block.latest_checkpoint_id && block.latest_checkpoint_id == checkpoint_id
                            ||  local_checkpoint_id == checkpoint_id && block.latest_checkpoint_id == checkpoint_id && local_checkpoint_id == 0
                        {
                            info!("Local checkpoint is latest: {}", checkpoint_id);
                            self.remote_latest_slot.store(block.compact.slot, Ordering::Relaxed);
                            ret.is_synced = true;
                            return Ok(ret);
                        }
                        Ok(ret)
                    }
                    Err(err) => {
                        error!(?checkpoint_id, ?err, "Error sync checkpoint");
                        sync_ctx.store.rollback(checkpoint_id)?;
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
        build_ctx: &ConcreteRealmProcessorContext,
        next_checkpoint_id: u64,
    ) -> anyhow::Result<ProvingJobDataId> {
        build_ctx.build_block().await.map(|job_id|ProvingJobDataId::new(next_checkpoint_id, job_id))
    }

    fn validate_slot(&self) -> anyhow::Result<()> {
        let slot = self.slot_timer.get_current_slot();
        if !self.is_current_slot() {
            bail!("Not in current slot, slot: {}, remote latest slot: {}", slot, self.remote_latest_slot.load(Ordering::Relaxed))
        }

        // if !self.slot_timer.is_can_reach_to_next_slot() {
        //     bail!("Not reach to next slot")
        // }
        Ok(())
    }

    fn is_current_slot(&self) -> bool {
        self.remote_latest_slot.load(Ordering::Relaxed) == 0 || self.slot_timer.get_current_slot() >= self.remote_latest_slot.load(Ordering::Relaxed)
    }
    pub async fn get_local_latest_checkpoint_id(&self) -> anyhow::Result<u64> {
        let state = self
            .store
            .get_latest_l2_block_state()
            .await
            .map_err(|err| anyhow!("Error getting latest l2 block state: {:?}", err))?;
        Ok(state.checkpoint_id)
    }

    pub async fn initialize_store(store: &QEDStore, genesis_config: Option<GenesisConfig<GoldilocksField>>, realm_id: u32) -> anyhow::Result<()> {
        if let Some(genesis_config) = genesis_config {
            info!("Processing genesis state for realm {}", realm_id);

            let realm_start_user = (realm_id as u64) * USERS_PER_REALM;
            let realm_end_user = ((realm_id + 1) as u64) * USERS_PER_REALM;

            let mut user_contract_states: HashMap<u64, HashMap<u64, Vec<(u64, QHashOut<F>)>>> = HashMap::new();
            let mut user_id_to_register_id: HashMap<u64, u64> = HashMap::new();

            for (contract_id, users) in genesis_config.get_all_contracts() {
                for (register_id, user_state) in users {
                    let user_id = get_user_id_from_registration_id(*register_id);

                    if user_id >= realm_start_user && user_id < realm_end_user {
                        user_id_to_register_id.insert(user_id, *register_id);

                        let mut contract_slots = Vec::new();
                        for (slot_id, slot_value) in &user_state.slots {
                            contract_slots.push((*slot_id, *slot_value));
                        }

                        if !contract_slots.is_empty() {
                            user_contract_states.entry(user_id).or_insert_with(HashMap::new)
                                .insert(*contract_id, contract_slots);
                        }
                    }
                }
            }

            let genesis_users = genesis_config.get_genesis_users();

            let mut realm_updates = Vec::new();
            for (user_id, contracts) in user_contract_states {
                let register_id = user_id_to_register_id[&user_id];

                let mut user_contract_tree_root = QEDHasher::get_zero_hash(GLOBAL_CONTRACT_TREE_HEIGHT.into());

                for (contract_id, slots) in contracts {
                    let mut contract_state_root = QEDHasher::get_zero_hash(MAX_CONTRACT_STATE_TREE_HEIGHT.into());
                    for (slot_id, slot_value) in slots {
                        contract_state_root = store.set_user_state_tree_leaf_hash(0, user_id, contract_id as u32, MAX_CONTRACT_STATE_TREE_HEIGHT, slot_id, slot_value)?.new_root;
                    }

                    user_contract_tree_root = store.set_user_contract_tree_leaf_hash(
                        0,
                        user_id,
                        contract_id as u32,
                        contract_state_root,
                    )?.new_root;
                }

                let user_leaf = QEDUserLeaf {
                    public_key: genesis_users[register_id as usize].get_public_key::<QEDHasher>(),
                    user_state_tree_root: user_contract_tree_root,
                    balance: F::ZERO,
                    nonce: F::ZERO,
                    last_checkpoint_id: F::ZERO,
                    event_index: F::ZERO,
                    user_id: F::from_canonical_u64(user_id),
                };

                let user_leaf_hash = user_leaf.qfhash::<QEDHasher>();

                store.set_user_leaf_data(0, &user_leaf)
                    .map_err(|e| anyhow::anyhow!("Failed to set user leaf data for user {}: {}", user_id, e))?;

                let realm_update = QMerkleNode {
                    key: SimpleMerkleNodeKey {
                        level: GLOBAL_USER_TREE_HEIGHT,
                        index: user_id,
                    },
                    value: user_leaf_hash,
                };
                realm_updates.push(realm_update);

                info!("✅ Genesis state set for user {} with UCT root {} (realm {})",
                      user_id, user_contract_tree_root, realm_id);
            }

            store.injest_user_tree_nodes_imm(0, COORDINATOR_USER_TREE_HEIGHT, &realm_updates).await?;

            info!("Genesis state initialization completed for realm {}", realm_id);
        }

        Ok(())
    }

    async fn initialize_genesis_state(&self) -> anyhow::Result<()> {
        let genesis_config = GenesisConfig::from_path(&self.config_path)?;
        Self::initialize_store(&self.store, genesis_config, self.realm_config.realm_id).await
    }

    async fn backup_task(mut rx: mpsc::UnboundedReceiver<RealmBackupRequest>, backup_client: RealmS3BackupClient , realm_id: u32) {
        info!("🚀 Realm backup task started");
        while let Some(request) = rx.recv().await {
            let RealmBackupRequest { checkpoint_id, pair_to_set, removed_keys } = request;
            // Retry up to 3 times with 1 second delay
            for retry_count in 0..=3 {
                match super::backup::create_realm_checkpoint_backup(
                    realm_id,
                    checkpoint_id,
                    pair_to_set.clone(),
                    removed_keys.clone(),
                ).await {
                    Ok(backup) => {
                        match backup_client.backup_checkpoint(&backup).await {
                            Ok(_) => {
                                info!("✅ Realm checkpoint {} backup succeeded", checkpoint_id);
                                break;
                            }
                            Err(e) if retry_count < 3 => {
                                warn!("⚠️ Realm backup retry {}/3 for checkpoint {}: {}",
                                    retry_count + 1, checkpoint_id, e);
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                            Err(e) => {
                                error!("❌ Realm backup final failure for checkpoint {}: {}",
                                    checkpoint_id, e);
                            }
                        }
                    }
                    Err(e) if retry_count < 3 => {
                        warn!("⚠️ Realm backup creation retry {}/3 for checkpoint {}: {}",
                            retry_count + 1, checkpoint_id, e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                    Err(e) => {
                        error!("❌ Failed to create realm backup for checkpoint {}: {}",
                            checkpoint_id, e);
                        break;
                    }
                }
            }
        }
        warn!("🔚 Realm backup task stopped");
    }

    async fn backup_checkpoint(
        &self,
        backup_client: &RealmS3BackupClient,
        checkpoint_id: u64,
        pair_to_set: Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>,
        removed_keys: Vec<Vec<u8>>,
    ) {
        let pair_to_set = pair_to_set.into_iter().map(|pair| (pair.key, pair.value)).collect();
        try_backup_realm_checkpoint(backup_client, checkpoint_id, pair_to_set, removed_keys).await;
    }
}

impl Retryable for RealmProcessor {}

#[derive(Debug, Clone)]
pub struct SyncCheckpointResult {
    pub checkpoint_id: u64,
    pub latest_checkpoint_id: u64,
    pub slot: u64,
    pub is_synced: bool,
    pub realm_root: QHashOut<F>,
}
