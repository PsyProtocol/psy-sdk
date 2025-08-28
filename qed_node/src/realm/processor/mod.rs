mod slot_phase;

use std::ops::Deref;
use crate::common::verifier::get_cached_generic_verifier;
use crate::realm::config::RealmNodeConfig;
use crate::realm::state::processor::{RealmConfig, RealmProcessorContext};
use crate::realm::{C, D, F};
use qed_core::config::network_constants::{COORDINATOR_USER_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT};
use qed_core::job::history_queue::{
    CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm,
};
use qed_core::job::id::ProvingJobDataId;
use qed_core::job::worker_queue::WorkerEventTransmitterAsyncImm;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_store::queue::QPendingUserStoreAsyncImm;
use qed_store::queue::task_queue::{QProvingTaskStore, QProvingTaskStoreImpl};
use qed_store::store::QEDStore;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use anyhow::{anyhow, bail};
use futures::future::{err, ok};
use tower_http::follow_redirect::policy::PolicyExt;
use tracing::{debug, error, info, warn};
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
    pub store: Arc<JournalStore<QEDStore>>,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub task_store: Arc<QProvingTaskStoreImpl>,
    pub slot_timer: SlotTimer<LocalClock>,
    pub remote_latest_slot: u64,
    pub config_path: String,
}

pub async fn run_realm_processor(config: RealmNodeConfig) -> anyhow::Result<()> {
    let mut realm_processor = RealmProcessor::new(config).await?;

    realm_processor.initialize_genesis_state().await?;

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
            slot_timer: SlotTimer::new(LocalClock),
            remote_latest_slot: 0,
            config_path: config.config_path.clone(),
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
        ).await?;
        info!("Realm Processor started");

        // Check for incomplete consumption state on startup
        if let Ok(Some(last_state)) = context.sync_queue.get_last_peek_offset().await {
            info!("🔄 Found incomplete consumption state for checkpoint {} on startup", last_state.checkpoint_id);
        }

        if let Ok(local_latest_l2_block_state) = context.store.get_latest_l2_block_state().await {
            info!(
                "local_latest_l2_block_state: {:?}",
                local_latest_l2_block_state
            );

            let pending_users_count = context.sync_queue.get_pending_users_count().await?;
            info!("Found {} pending users in Redis queue during recovery", pending_users_count);
        }

        // Ensure checkpoint sync first
        self.ensure_checkpoint_sync(&mut context).await?;
        let slot_timer = self.slot_timer.clone();
        loop {
            tokio::select! {
                checkpoint_sync_result = self.ensure_checkpoint_sync(&mut context) => {
                    match checkpoint_sync_result {
                        Ok(true) => {
                            info!("Checkpoint sync completed");
                        }
                        Ok(false) => {
                            info!("No new checkpoint to sync");
                        }
                        Err(err) => {
                            error!("Checkpoint sync failed: {:?}", err);
                        }
                    }
                    continue;
                },
                slot = slot_timer.wait_for_next_slot() => {
                    info!("Next slot: {}", slot);
                }
            }

            // Build block based on slot timing
            if let Err(err) = self.validate_slot() {
                warn!("Error validating slot: {:?}", err);
                continue
            }

            let slot = self.slot_timer.get_current_slot();
            if let SlotPhase::BuildPhase(build_phase_start) = SlotPhase::get_build_phase(self.slot_timer.deref()){
                let current_timestamp = self.slot_timer.get_current_timestamp();
                if current_timestamp < build_phase_start {
                    let tt = build_phase_start - current_timestamp;
                    info!("Waiting for build phase to start: sleep {} ms, slot: {}", tt, slot);
                    tokio::time::sleep(Duration::from_millis(tt)).await;
                }
            }

            info!("Start building block");
            let local_latest_checkpoint_id = self.get_local_latest_l2_block_state().await?;
            let next_checkpoint_id = local_latest_checkpoint_id + 1;
            self.store.commit(local_latest_checkpoint_id)?;
            context.commit_offset().await?;
            let has_tasks = context.has_pending_tasks(next_checkpoint_id).await?;
            if !has_tasks {
                warn!("No, pending tasks for checkpoint {}, skipping block construction", next_checkpoint_id);
                continue;
            }
            let proving_data_job_id: ProvingJobDataId = match self.build_block(next_checkpoint_id, &mut context, &realm_qps).await {
                Ok(job_id) => job_id,
                Err(err) => {
                    error!("Error building block: {:?}, slot: {}", err, slot);
                    continue;
                }
            };
            info!("Pushing job id to queue: {:?}, slot: {}", proving_data_job_id, slot);
            self.sync_proof.chq_push_imm(proving_data_job_id).await?;
            // Send the job id to the channel for the next step
            // if let Err(err) = self.queue.cdq_push_imm(proving_data_job_id).await {
            //     error!("Error chq_push_imm: {:?}", err);
            // };
            info!("Pushing job to queueue done");
        }
    }

    async fn ensure_checkpoint_sync(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
    ) -> anyhow::Result<bool> {
        loop {
            match self.sync_checkpoint(context).await {
                Ok(true) => return Ok(true),  // Sync completed
                Err(err) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    bail!("Checkpoint sync attempt failed: {:?}", err)
                }
                _ => {
                    continue;
                }
            }
        }
    }

    pub async fn sync_checkpoint(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
    ) -> anyhow::Result<bool> {
        let (expected_checkpoint,local_checkpoint_id) = if let Ok(local_checkpoint_id) = self.get_local_latest_l2_block_state().await {
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

                info!("Checkpoint received checkpoint_id: {},latest_checkpoint_id: {} ,local_checkpoint_id: {}", checkpoint_id, block.latest_checkpoint_id, local_checkpoint_id);
                if local_checkpoint_id >= block.latest_checkpoint_id && local_checkpoint_id > 0 {
                    info!("Local checkpoint is latest");
                    self.remote_latest_slot = block.compact.slot;
                    return Ok(true);
                }
                if local_checkpoint_id >= checkpoint_id && local_checkpoint_id > 0 {
                    info!("Local checkpoint is up to date");
                    return Ok(false);
                }

                info!("Syncing checkpoint");
                match context.handle_checkpoint_sync(block.compact.clone()).await {
                    Ok(_) => {
                        info!(?checkpoint_id, "Sync to new checkpoint");
                        info!("Checkpoint sync reg users: {:?}", block.compact.registered_users);
                        self.store.commit(checkpoint_id)?;

                        // Check updated pending users count
                        let pending_users_count = context.sync_queue.get_pending_users_count().await?;
                        info!("Pending users count after checkpoint sync: {}", pending_users_count);

                        if local_checkpoint_id + 1 == block.latest_checkpoint_id && block.latest_checkpoint_id == checkpoint_id
                            ||  local_checkpoint_id == checkpoint_id && block.latest_checkpoint_id == checkpoint_id && local_checkpoint_id == 0
                        {
                            info!("Local checkpoint is latest");
                            self.remote_latest_slot = block.compact.slot;
                            return Ok(true);
                        }
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
        &self,
        next_checkpoint_id: u64,
        context: &ConcreteRealmProcessorContext,
        realm_qps: &ProofStoreRedisAsync,
    ) -> anyhow::Result<ProvingJobDataId> {
        let store = self.store.clone();
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
                    store.rollback(next_checkpoint_id)?;

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

        if !self.slot_timer.is_can_reach_to_next_slot() {
            bail!("Not reach to next slot")
        }
        Ok(())
    }

    fn is_current_slot(&self) -> bool {
        self.remote_latest_slot == 0 || self.slot_timer.get_current_slot() > self.remote_latest_slot
    }
    pub async fn get_local_latest_l2_block_state(&self) -> anyhow::Result<u64> {
        let state = self
            .store
            .get_latest_l2_block_state()
            .await
            .map_err(|err| anyhow!("Error getting latest l2 block state: {:?}", err))?;
        Ok(state.checkpoint_id)
    }

    async fn initialize_genesis_state(&mut self) -> anyhow::Result<()> {
        let config_path = &self.config_path;

        let genesis_config = match std::fs::read_to_string(config_path) {
            Ok(config_content) => {
                let config_value: serde_json::Value = serde_json::from_str(&config_content)?;
                if let Some(genesis_obj) = config_value.get("genesis") {
                    let genesis_config = qed_core::config::genesis::GenesisConfig::from_json(
                        &serde_json::to_string(genesis_obj)?
                    )?;
                    Some(genesis_config)
                } else {
                    warn!("No genesis configuration found in config file");
                    return Ok(());
                }
            }
            Err(e) => {
                warn!("Could not read config file {}: {}", config_path, e);
                return Ok(());
            }
        };

        if let Some(genesis_config) = genesis_config {
            info!("Processing genesis state for realm {}", self.realm_config.realm_id);

            use qed_core::config::network_constants::REALM_USER_TREE_HEIGHT;
            let users_per_realm = 1u64 << REALM_USER_TREE_HEIGHT;
            let realm_start_user = (self.realm_config.realm_id as u64) * users_per_realm;
            let realm_end_user = ((self.realm_config.realm_id + 1) as u64) * users_per_realm;

            use qed_data::qstore::uct_merkle_nodes::{CSTUserUpdate, CSTDeltaNodeKey, CSTDeltaNode};
            use qed_core::data::qhashout::QHashOut;
            use qed_crypto::hash::merkle::utils::{simple_merkle_tree::SimpleMerkleTree, common::{SimpleMerkleNode, SimpleMerkleNodeKey}};
            use qed_data::config::store_config::QEDHasher;
            use qed_core::config::network_constants::{MAX_CONTRACT_STATE_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT};
            use std::{str::FromStr, collections::HashMap};

            let mut user_contract_states: HashMap<u64, HashMap<u64, Vec<(u64, QHashOut<F>)>>> = HashMap::new();

            for (contract_id_str, users) in genesis_config.get_all_contracts() {
                let contract_id: u64 = contract_id_str.parse()
                    .map_err(|e| anyhow::anyhow!("Invalid contract ID {}: {}", contract_id_str, e))?;

                for (user_id_str, user_state) in users {
                    let user_id: u64 = user_id_str.parse()
                        .map_err(|e| anyhow::anyhow!("Invalid user ID {}: {}", user_id_str, e))?;

                    if user_id >= realm_start_user && user_id < realm_end_user {
                        let mut contract_slots = Vec::new();
                        for (slot_id_str, hex_value) in &user_state.slots {
                            let slot_id: u64 = slot_id_str.parse()
                                .map_err(|e| anyhow::anyhow!("Invalid slot ID {}: {}", slot_id_str, e))?;
                            let slot_value = QHashOut::<F>::from_str(hex_value)
                                .map_err(|e| anyhow::anyhow!("Invalid hex value {}: {}", hex_value, e))?;

                            info!("  Setting genesis slot {}: {} for user {} contract {}",
                                  slot_id, hex_value, user_id, contract_id);
                            contract_slots.push((slot_id, slot_value));
                        }

                        if !contract_slots.is_empty() {
                            user_contract_states.entry(user_id).or_insert_with(HashMap::new)
                                .insert(contract_id, contract_slots);
                        }
                    }
                }
            }

            for (user_id, contracts) in user_contract_states {
                let mut all_cst_updates = Vec::new();
                let mut user_contract_tree = SimpleMerkleTree::<QEDHasher, QHashOut<F>>::new(GLOBAL_USER_TREE_HEIGHT);

                for (contract_id, slots) in contracts {
                    let mut contract_state_tree = SimpleMerkleTree::<QEDHasher, QHashOut<F>>::new(MAX_CONTRACT_STATE_TREE_HEIGHT);

                    for (slot_id, slot_value) in slots {
                        let slot_key = SimpleMerkleNodeKey::new(MAX_CONTRACT_STATE_TREE_HEIGHT, slot_id);
                        contract_state_tree.set_node_value(slot_key, slot_value);

                        let cst_key = CSTDeltaNodeKey::new(contract_id as u32, MAX_CONTRACT_STATE_TREE_HEIGHT, slot_id);
                        let cst_update = CSTDeltaNode { key: cst_key, value: slot_value };
                        all_cst_updates.push(cst_update);
                    }

                    let contract_state_root = contract_state_tree.get_root();
                    let contract_key = SimpleMerkleNodeKey::new(GLOBAL_USER_TREE_HEIGHT, contract_id);
                    user_contract_tree.set_node_value(contract_key, contract_state_root);

                    info!("  Contract {} state root: {} for user {}",
                          contract_id, contract_state_root, user_id);
                }

                let user_contract_tree_root = user_contract_tree.get_root();
                let user_index_in_realm = user_id % users_per_realm;

                let mut uct_updates = Vec::new();
                let uct_key = SimpleMerkleNodeKey::new(COORDINATOR_USER_TREE_HEIGHT, user_index_in_realm);
                let uct_update = SimpleMerkleNode { key: uct_key, value: user_contract_tree_root };
                uct_updates.push(uct_update);

                let user_update = CSTUserUpdate {
                    checkpoint_id: 0,
                    user_id,
                    uct_updates,
                    updates: all_cst_updates,
                };

                use qed_store::node::realm::QEDRealmStoreWriterAsyncImm;
                self.store.injest_checked_cst_nodes_imm(&[user_update]).await
                    .map_err(|e| anyhow::anyhow!("Failed to set genesis state for user {}: {}", user_id, e))?;

                info!("✅ Genesis state set for user {} with UCT root {} (realm {})",
                      user_id, user_contract_tree_root, self.realm_config.realm_id);
            }

            info!("Genesis state initialization completed for realm {}", self.realm_config.realm_id);
        }

        Ok(())
    }
}

impl Retryable for RealmProcessor {}
