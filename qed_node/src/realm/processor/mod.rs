pub mod processor_v2;
use plonky2::plonk::proof::ProofWithPublicInputs;
pub use processor_v2::*;
use qed_data::guta::api::{GUTARealmCheckpointResult, SubmitGUTARealmResultAPINoProofInput};
use qed_data::models::checkpoint::sync_info::CheckpointError;
use serde::{Deserialize, Serialize};

use crate::common::verifier::get_cached_generic_verifier;
use crate::common_v2::traits::realm::CoordinatorClient;
use crate::coordinator::client_v2::ConcreteCoordinatorClient;
use crate::realm::config::RealmNodeConfig;
use crate::realm::state::processor::{RealmConfig, RealmProcessorContext};
use crate::realm::{C, D, F};
use qed_core::config::network_constants::{COORDINATOR_USER_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT};
use qed_core::job::id::ProvingJobDataId;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_crypto::hash::merkle::utils::common::{QMerkleNode, SimpleMerkleNodeKey};
use qed_crypto::hash::traits::hasher::MerkleZeroHasher;
use qed_data::config::genesis_config::GenesisConfig;
use qed_store::queue::QPendingUserStoreAsyncImm;
use qed_store::queue::task_queue::QProvingTaskStoreImpl;
use qed_store::store::QEDStore;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::time::Instant;
use anyhow::{anyhow, bail};
use tracing::{error, info, trace, warn};
use qed_core::data::qhashout::QHashOut;
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::queue::ProofStoreRedisAsync;
use qed_store::store::journal::{Journal, JournalStore, BackupJournalStore, BackupRequest};
use crate::common::clock::SlotTimer;
use crate::common::slot::{LocalClock, Slot};
use qed_core::config::network_constants::USERS_PER_REALM;
use qed_data::config::store_config::QEDHasher;
use qed_core::config::network_constants::{MAX_CONTRACT_STATE_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT};
use qed_data::qdata::user::QEDUserLeaf;
use qed_crypto::hash::traits::qhashable::QFieldHashable;
use qed_store::node::realm::QEDRealmStoreWriterAsyncImm;
use qed_crypto::common::user_id::get_user_id_from_registration_id;
use plonky2::field::types::Field;
use qed_data::traits::qdatastore::{qtreedata::QTreeDataStoreWriterSync, qmetadata::QMetaDataStoreWriterSync};
use std::collections::HashMap;
use plonky2::field::goldilocks_field::GoldilocksField;
use super::backup::RealmS3BackupClient;
use tokio::sync::mpsc;
use kvq::traits::{KVQBinaryStore, KVQPair};
use qed_store::queue::redis_queue::QueueOffsetState;
use crate::realm::state::edge_queue_helper::RealmEdgeQueueHelper;
use crate::realm::state::queue_factory::QueueFactory;
use qed_data::qdata::hash_key::Hash4x64Key;
use qed_data::qdata::realm_snapshot_key::RealmSnapshotKey;
use qed_data::config::store_config::{REALM_SNAPSHOT_TABLE_TYPE, REALM_ROOT_VERSION_TABLE_TYPE};
use kvq::traits::KVQSerializable;

const CHECKPOINT_BATCH_SIZE: u64 = 25; // sync info batch size

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u64,
    pub checkpoint_id: u64,
    pub cache: Vec<u8>,
    pub root: QHashOut<F>,
    pub pre_root: QHashOut<F>,
    pub queue_offset_state: Vec<QueueOffsetState>,
}

type Context = RealmProcessorContext<
    BackupJournalStore<JournalStore<QEDStore>>,
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
    pub queue_helper: Arc<RealmEdgeQueueHelper<F>>,
    pub client: Arc<ConcreteCoordinatorClient>,
    pub backup_tx: Option<mpsc::UnboundedSender<BackupRequest>>,
}

pub async fn run_realm_processor(config: RealmNodeConfig) -> anyhow::Result<()> {
    let realm_processor = RealmProcessor::new(config).await?;
    let _ = realm_processor.start().await?;
    Ok(())
}

impl RealmProcessor {
    pub async fn new(config: RealmNodeConfig) -> anyhow::Result<Self> {
        info!("Realm Processor Config: {:?}", config);
        let task_store = QProvingTaskStoreImpl::new(
            &config.redis.redis_uri.as_str(),
            config.redis.pool_size.unwrap_or(10),
            &config.queue.queue_biz_key
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

        // Initialize backup channel and task
        let backup_tx = match RealmS3BackupClient::new_from_env(config.realm.realm_id).await {
            Ok(client) => {
                info!("✅ S3 backup client initialized");
                let (tx, rx) = mpsc::unbounded_channel();
                let realm_id = config.realm.realm_id;
                tokio::spawn(async move {
                    Self::backup_task(rx, client, realm_id).await;
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
            queue_helper: Arc::new(queue_helper),
            client: coordinator_client,
            backup_tx,
        };
        Ok(processor)
    }

    async fn context(&self) -> anyhow::Result<Context> {
        let realm_qps = Arc::new(self.sync_proof.clone());
        let journal_store = JournalStore::new(self.store.clone());
        
        // Wrap journal store with backup functionality if backup is enabled
        let backup_journal_store = if let Some(ref backup_tx) = self.backup_tx {
            BackupJournalStore::new_with_backup(journal_store, backup_tx.clone())
        } else {
            BackupJournalStore::new(journal_store)
        };
        
        RealmProcessorContext::<
            BackupJournalStore<JournalStore<QEDStore>>,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
            ProofStoreRedisAsync,
            QProvingTaskStoreImpl,
        >::new(
            self.realm_config,
            self.max_processed_end_caps_per_block,
            backup_journal_store,
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            self.task_store.clone(),
            self.proof_verifier.clone(),
        ).await
    }

    async fn get_realm_root_diff(&self, build_ctx: &Context, checkpoint_id: u64) -> anyhow::Result<(QHashOut<F>, QHashOut<F>)> {
        let pre_realm_root = self.get_realm_root(&self.store, checkpoint_id.saturating_sub(1)).await?;
        let realm_root = self.get_realm_root(&build_ctx.store, checkpoint_id).await?;
        Ok((pre_realm_root, realm_root))
    }

    async fn get_realm_root(&self, store: &dyn QEDRealmStoreReaderAsync<F>, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        let realm_root = store.get_user_sub_tree_merkle_proof(
            checkpoint_id,
            0,
            COORDINATOR_USER_TREE_HEIGHT,
            self.realm_config.realm_id as u64,
        ).await?;
        Ok(realm_root.value)
    }

    pub async fn start(self) -> anyhow::Result<()> {
        info!("Realm Processor starting");
        self.initialize().await?;
        self.run().await?;
        Ok(())
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        let ctx = self.context().await?;
        if let Err(e) = self.store.get_latest_l2_block_state().await {
            warn!("Failed to get latest L2 block state: {:?}", e);
            if let Ok(CheckpointError::NotFound) = e.downcast::<CheckpointError>(){
                // initialize genesis state
                self.initialize_genesis_state(&ctx).await?;
                let block0 = self.client.get_checkpoint_sync_info(self.realm_config.realm_id, 0).await?;
                self.handle_sync_info(&ctx, block0).await?;
                ctx.commit(0, vec![]).await?;
            }
        }
        Ok(())
    }

    async fn run(&self) -> anyhow::Result<()> {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Err(err) =  self.build().await {
                error!("Build error: {:?}", err);
            }
        }
    }

    async fn confirm_checkpoint(&self, realm_root: QHashOut<F>, checkpoint_id: u64, latest_checkpoint_id: u64) -> anyhow::Result<()> {
        let snapshot = self.load_snapshot(realm_root)?;
        let build_ctx = self.context().await?;
        build_ctx.store.restore_cache(snapshot.cache)?;
        trace!("synced checkpoint id: {}, latest checkpoint id: {}, realm root: {}", checkpoint_id, latest_checkpoint_id, realm_root);
        build_ctx.commit(checkpoint_id, snapshot.queue_offset_state).await.map_err(|e| anyhow!("Failed to commit checkpoint {}: {}", checkpoint_id, e))?;
        info!("Commit checkpoint {}, latest_checkpoint_id: {}", checkpoint_id, latest_checkpoint_id);
        Ok(())
    }

    async fn build(&self) -> anyhow::Result<()> {
        if !self.is_state_normal().await? {
            warn!("State is not normal, skipping");
            return Ok(());
        }

        self.sync_to_latest().await?;

        if let Err(err) = self.validate_slot() {
            warn!("Error validating slot: {:?}", err);
            return Ok(());
        }

        let slot = self.slot_timer.get_current_slot();
        trace!("Next slot: {}", slot);
        let local_latest_checkpoint_id = self.get_local_latest_checkpoint_id().await?;
        let next_checkpoint_id = local_latest_checkpoint_id + 1;
        let build_ctx = &self.context().await?;
        let has_tasks = build_ctx.has_pending_guta_tasks(next_checkpoint_id).await? || build_ctx.has_pending_user_tasks().await?;
        if !has_tasks {
            trace!("No pending tasks for checkpoint {}, skipping block construction", next_checkpoint_id);
            return Ok(());
        }
        let now = Instant::now();
        info!("Start building block checkpoint: {}, slot: {}", next_checkpoint_id, slot);
        match self.build_block(build_ctx, next_checkpoint_id, slot).await {
            Ok((job_id, off_state)) => {
                self.save_snapshot(build_ctx, next_checkpoint_id, off_state).await?;
                self.submit_guta(build_ctx, job_id).await?;
                info!("Build complete checkpoint: {}, slot: {}, cost time: {:?}", next_checkpoint_id, slot, now.elapsed());
            }
            Err(err) => {
                // Rollback database changes
                build_ctx.rollback(next_checkpoint_id).await?;
                error!("Rollback: build block failed for checkpoint {}: {:?}", next_checkpoint_id, err);
            }
        }
        Ok(())
    }

    async fn save_snapshot(&self, build_ctx: &Context, checkpoint_id: u64, offset_state: Vec<QueueOffsetState>) -> anyhow::Result<()> {
        let (pre_realm_root, realm_root) = self.get_realm_root_diff(build_ctx, checkpoint_id).await?;
        
        if let Some(cache) = build_ctx.store.get_cache()? {
            // Get or initialize version
            let version = self.get_realm_root_version(realm_root)?;
            
            let snapshot = Snapshot {
                version,
                checkpoint_id,
                cache,
                root: realm_root.clone(),
                pre_root: pre_realm_root,
                queue_offset_state: offset_state,
            };
            
            // Serialize data
            let snapshot_value = bincode::serialize(&snapshot)?;
            let version_value = bincode::serialize(&(version + 1))?;
            
            // Use structured keys
            let snapshot_key = self.snapshot_key(realm_root, version)?;
            let version_key = self.version_key(realm_root)?;
            
            let set = vec![
                KVQPair {
                    key: &version_key,
                    value: &version_value,
                },
                KVQPair {
                    key: &snapshot_key,
                    value: &snapshot_value,
                },
            ];
            self.store.set_and_delete_many(&set, &*vec![])?;
        }
        Ok(())
    }
    
    fn load_snapshot(&self, realm_root: QHashOut<F>) -> anyhow::Result<Snapshot> {
        // Get version
        let version_key = self.version_key(realm_root)?;
        let version_data = self.store.get_exact(&version_key)?;
        let version: u64 = bincode::deserialize(&version_data)?;
        
        // Get snapshot
        let snapshot_key = self.snapshot_key(realm_root, version)?;
        let snapshot_data = self.store.get_exact(&snapshot_key)?;
        let snapshot: Snapshot = bincode::deserialize(&snapshot_data)?;
        Ok(snapshot)
    }

    fn snapshot_key(&self, realm_root: QHashOut<F>, version: u64) -> anyhow::Result<Vec<u8>> {
        let snapshot_key = RealmSnapshotKey::<REALM_SNAPSHOT_TABLE_TYPE>::new(realm_root, version).to_bytes()?;
        Ok(snapshot_key)
    }

    fn version_key(&self, realm_root: QHashOut<F>) -> anyhow::Result<Vec<u8>> {
        let version_key = Hash4x64Key::<REALM_ROOT_VERSION_TABLE_TYPE>::from(realm_root).to_bytes()?;
        Ok(version_key)
    }
    
    fn is_candidate(&self, realm_root: QHashOut<F>) -> anyhow::Result<bool> {        
        let version_key = self.version_key(realm_root)?;
        let version = self.store.get_exact_if_exists(&version_key)?;
        Ok(version.is_some())
    }

    fn get_realm_root_version(&self, realm_root: QHashOut<F>) -> anyhow::Result<u64> {
        let version_key = self.version_key(realm_root)?;
        match self.store.get_exact_if_exists(&version_key)? {
            Some(data) => {
                let version: u64 = bincode::deserialize(&data)?;
                Ok(version)
            }
            None => Ok(0)
        }
    }

    async fn submit_guta(
        &self,
        build_ctx: &Context,
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
            proof_id: realm_result.proof_id.get_output_id()
        };
        self.client.submit_guta_v1(&input, &bincode::serialize(&proof)?, input.realm_id).await
    }

    async fn sync_to_latest(&self) -> anyhow::Result<()> {
        loop {
            let has_pending_guta = self.client.has_pending_guta(self.realm_config.realm_id as u32).await?;
            if has_pending_guta {
                warn!("Has pending guta, skipping sync to latest");
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
            loop {
                let local_checkpoint_id = self.get_local_latest_checkpoint_id().await?;
                let latest_sync_info = self.get_remote_latest_checkpoint_sync_info().await?;
                if local_checkpoint_id == latest_sync_info.latest_checkpoint_id {
                    info!("Local checkpoint is up to date");
                    self.remote_latest_slot.store(latest_sync_info.compact.slot, Ordering::Relaxed);
                    return Ok(());
                }
                let start = local_checkpoint_id;
                let end = local_checkpoint_id + CHECKPOINT_BATCH_SIZE;
                self.sync_checkpoint_range(start, end).await?;
            }
        }
    }

    pub async fn handle_sync_info(
        &self,
        sync_ctx: &Context,
        block: CheckpointSyncInfo<F>,
    ) -> anyhow::Result<()> {
        let sync_checkpoint_id = block.compact.l2_block_state.checkpoint_id;
        match sync_ctx.handle_checkpoint_sync(block.compact.clone()).await {
            Ok(_) => {
                info!("Checkpoint {} sync reg users len: {}", sync_checkpoint_id, block.compact.registered_users.len());

                let pending_users_count = sync_ctx.sync_queue.get_pending_users_count().await?;
                trace!("Pending users count after checkpoint sync: {}", pending_users_count);
                Ok(())
            }
            Err(err) => {
                error!(?sync_checkpoint_id, ?err, "Error sync checkpoint");
                Err(err)
            }
        }
    }

    async fn sync_checkpoint_range(&self, start: u64, end: u64) -> anyhow::Result<()> {
        // [start, end]
        let mut sync_infos = self.fetch_remote_sync_infos(start, end).await.map_err(|e| anyhow!("Fetch remote sync infos error: {:?}", e))?;
        if sync_infos.is_empty() {
            return Ok(());
        }
        let start_sync_info = sync_infos.remove(0);
        let mut local_realm_root = start_sync_info.realm_root;
        let mut local_checkpoint_id = start_sync_info.compact.l2_block_state.checkpoint_id;

        for block in sync_infos {
            let sync_checkpoint_id = block.compact.l2_block_state.checkpoint_id;
            let latest_checkpoint_id = block.latest_checkpoint_id;
            let remote_realm_root = block.realm_root;
            info!("Checkpoint received checkpoint_id: {},latest_checkpoint_id: {} ,local_checkpoint_id: {}, local_realm_root: {}, remote_realm_root: {}",
                sync_checkpoint_id, latest_checkpoint_id, local_checkpoint_id, local_realm_root, remote_realm_root);
            self.handle_sync_info(&self.context().await?, block).await.map_err(|e| anyhow!("Handle sync info attempt failed: {:?}", e))?;
            // confirm pending checkpoint
            if local_realm_root != remote_realm_root && self.is_candidate(remote_realm_root.clone())? {
                self.confirm_checkpoint(remote_realm_root.clone(), sync_checkpoint_id, latest_checkpoint_id).await?;
                local_realm_root = remote_realm_root;
            }
            local_checkpoint_id = sync_checkpoint_id;
        }
        
        Ok(())
    }

    async fn get_remote_latest_checkpoint_sync_info(&self) -> anyhow::Result<CheckpointSyncInfo<F>> {
        self.client.get_latest_checkpoint_sync_info(self.realm_config.realm_id as u32).await
    }

    async fn fetch_remote_sync_infos(&self, start: u64, end: u64) -> anyhow::Result<Vec<CheckpointSyncInfo<F>>> {
        self.client.get_latest_block_updates_from_coordinator(self.realm_config.realm_id as u64, start, end).await
    }

    async fn is_state_normal(&self) -> anyhow::Result<bool> {
        if let Ok(checkpoint_id) = self.get_local_latest_checkpoint_id().await {
            let realm_root = self.get_realm_root(&self.store, checkpoint_id).await?;
            return self.is_candidate(realm_root);
        }
        Ok(false)
    }

    pub async fn build_block(
        &self,
        build_ctx: &Context,
        next_checkpoint_id: u64,
        slot: u64,
    ) -> anyhow::Result<(ProvingJobDataId, Vec<QueueOffsetState>)> {
        build_ctx.build_block(next_checkpoint_id, slot).await.map(|(job_id,state)|(ProvingJobDataId::new(next_checkpoint_id, job_id),state))
    }

    fn validate_slot(&self) -> anyhow::Result<()> {
        let slot = self.slot_timer.get_current_slot();
        if !self.is_current_slot() {
            bail!("Not in current slot, slot: {}, remote latest slot: {}", slot, self.remote_latest_slot.load(Ordering::Relaxed))
        }
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

    pub async fn initialize_store(store: Arc<dyn KVQBinaryStore>, genesis_config: Option<GenesisConfig<GoldilocksField>>, realm_id: u32) -> anyhow::Result<()> {
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

            if !realm_updates.is_empty() {
                store.injest_user_tree_nodes_imm(0, COORDINATOR_USER_TREE_HEIGHT, &realm_updates).await?;
                info!("✅ Genesis state initialization completed for realm {} with {} users", realm_id, realm_updates.len());
            } else {
                info!("⚠️ Genesis state initialization skipped for realm {} (no users assigned)", realm_id);
            }
        }

        Ok(())
    }

    async fn initialize_genesis_state(&self, ctx: &Context) -> anyhow::Result<()> {
        let genesis_config = GenesisConfig::from_path(&self.config_path)?;
        let store = Arc::new(ctx.store.clone());
        Self::initialize_store(store, genesis_config, self.realm_config.realm_id).await
    }

    async fn backup_task(mut rx: mpsc::UnboundedReceiver<BackupRequest>, backup_client: RealmS3BackupClient, realm_id: u32) {
        info!("🚀 Realm backup task started");
        while let Some(request) = rx.recv().await {
            let BackupRequest { checkpoint_id, pair_to_set, removed_keys } = request;
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
}