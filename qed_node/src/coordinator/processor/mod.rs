use super::args::CoordinatorProcessorArgs;
use crate::common::verifier::get_cached_generic_verifier;
use crate::coordinator::state::processor::CoordinatorConfig;
use crate::coordinator::state::processor::CoordinatorProcessorContext;
use anyhow::{bail, Context};
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::field::types::Field;
use qed_core::config::genesis::GenesisConfig;
use qed_core::config::network_constants::MAX_CONTRACT_STATE_TREE_HEIGHT;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::worker_queue::WorkerEventReceiverAsyncImm;
use qed_core::job::{
    drain_queue::CheckpointDrainQueueConsumerAsyncImm,
    history_queue::{CheckpointHistoryQueueEmitterAsyncImm, CheckpointHistoryQueueConsumerAsyncImm},
    traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
    worker_queue::WorkerEventTransmitterAsyncImm,
};
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_data::qdata::contract::QEDContractLeaf;
use qed_data::{
    config::store_config::QEDFelt, traits::qdatastore::qtreedata::QEDComboDataStoreReaderWriterSync,
};
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_store::node::coordinator::{
    QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm,
};
use qed_store::queue::new_redis_async_pool;
use qed_store::queue::ProofStoreFred;
use qed_store::queue::ProofStoreRedisAsync;
use qed_store::queue::rsmq_queue::CEQueueNotification;
use qed_core::config::network_constants::COORDINATOR_TO_REALM_CHANNEL;
use qed_store::store::QEDStore;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;
use std::time::Duration;

use qed_store::store::journal::{Journal, JournalStore};
use std::sync::Arc;
use tokio::time::{sleep_until, Instant};
use tracing::{debug, error, info, warn};
use serde_json;
use qed_store::queue::task_queue::{QProvingTaskStore, QProvingTaskStoreImpl};
use qed_store::queue::redis_queue::{CheckpointDrainQueueConsumerAsyncImmWithPosition, NotificationQueue};
use crate::common::clock::SlotTimer;
use crate::common::retry::Retryable;
use crate::common::slot;
use crate::common::slot::{LocalClock, Slot};
use crate::realm::RealmProcessor;
use qed_crypto::hash::merkle::utils::common::{QMerkleNode, SimpleMerkleNodeKey};

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = QEDFelt;

pub struct CoordinatorProcessNode<
    JL: Journal,
    SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
    DQ: CheckpointDrainQueueConsumerAsyncImmWithPosition,
    HQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm + NotificationQueue<CEQueueNotification>,
    WQ: WorkerEventTransmitterAsyncImm,
    PS: QProofStoreAsyncImm + QProofStoreWriterAsyncImm + QProofStoreReaderAsync,
    ER: WorkerEventReceiverAsyncImm,
    TS: QProvingTaskStore,
> {
    pub ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS, TS>,
    pub journal_store: JL,
    pub edge_command_queue: Arc<HQ>,
    pub proof_store: PS,
    pub event_receiver: ER,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
    pub task_store: Arc<QProvingTaskStoreImpl>,
}

impl<
        JL: Journal,
        SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueConsumerAsyncImmWithPosition,
        HQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm + NotificationQueue<CEQueueNotification>,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm,
        ER: WorkerEventReceiverAsyncImm,
        TS: QProvingTaskStore,
    > CoordinatorProcessNode<JL, SR, DQ, HQ, WQ, PS, ER, TS>
{
    pub fn new(
        ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS, TS>,
        journal_store: JL,
        edge_command_queue: Arc<HQ>,
        proof_store: PS,
        event_receiver: ER,
        proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
        coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
        task_store: Arc<QProvingTaskStoreImpl>,
    ) -> Self {
        Self {
            ctx,
            journal_store,
            edge_command_queue,
            proof_store,
            event_receiver,
            proof_verifier,
            coordinator_worker_circuits,
            task_store,
        }
    }

    pub async fn wait_for_produce_block(&mut self) -> anyhow::Result<bool> {
        // Get current checkpoint to listen from
        let latest_l2_block_state = self.ctx.store.get_latest_l2_block_state().await?;
        let notify_message = self.edge_command_queue.consume_item(COORDINATOR_TO_REALM_CHANNEL).await?;

        let latest_l2_block_state = self.ctx.store.get_latest_l2_block_state().await?;

        let CEQueueNotification::StartProduceBlock { next_checkpoint } = notify_message;
        debug!("coordinator: wait_for_produce_block: next_checkpoint: {}, latest_l2_block_state.checkpoint_id: {}",
            next_checkpoint, latest_l2_block_state.checkpoint_id);

        match next_checkpoint.cmp(&latest_l2_block_state.checkpoint_id) {
            std::cmp::Ordering::Equal => {
                info!("✅ Building new block for checkpoint {}", next_checkpoint);
                // No need to delete from history queue, it's already processed
                return Ok(false);
            }
            std::cmp::Ordering::Less => {
                warn!(
                    "⚠️ Outdated checkpoint {}, current {}",
                    next_checkpoint, latest_l2_block_state.checkpoint_id
                );
                // No need to delete from history queue, it's already processed
                return Ok(false);
            }
            std::cmp::Ordering::Greater
                if next_checkpoint - latest_l2_block_state.checkpoint_id > 1 =>
            {
                warn!(
                    "🚧 Future checkpoint {} too far ahead of {}",
                    next_checkpoint, latest_l2_block_state.checkpoint_id
                );
                return Ok(false);
            }
            std::cmp::Ordering::Greater => {
                return Ok(true);
            }
        }
    }

    pub async fn wait_for_make_block(&mut self) -> bool {
        match self.wait_for_produce_block().await {
            Ok(true) => {
                info!("✅ Successfully wait for produce block");
                true
            }
            Ok(false) => {
                info!("⚠️ No pending tasks, waiting for next checkpoint");
                tokio::time::sleep(Duration::from_millis(slot::SLOT_SIZE/2)).await;
                false
            }
            Err(e) => {
                error!("❌ Error waiting for produce block: {:?}", e);
                tokio::time::sleep(Duration::from_millis(slot::SLOT_SIZE/2)).await;
                false
            }
        }
    }
}

impl
    CoordinatorProcessNode<
        JournalStore<QEDStore>,
        JournalStore<QEDStore>,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        ProofStoreRedisAsync,
        QProvingTaskStoreImpl,
    >
{
    pub async fn new_with_config(cp_config: CoordinatorProcessorArgs) -> anyhow::Result<Self> {
        let bb8_pool =
            new_redis_async_pool(&cp_config.redis_uri, cp_config.redis_pool_size as usize).await?;
        info!("🐶 redis pool initialized");
        let task_store = Arc::new(QProvingTaskStoreImpl::new(&cp_config.redis_uri, cp_config.redis_pool_size as usize)
            .await?);
        let q = ProofStoreRedisAsync::new(
            bb8_pool,
            cp_config.queue_args.queue_biz_key.clone(),
        )
        .await?;

        let qed_store = QEDStore::from_backend(cp_config.backend.to_backend()).await?;
        let qed_store = JournalStore::new(qed_store);

        // Load genesis configuration from config file
        let genesis_config = match std::fs::read_to_string(&cp_config.config_path) {
            Ok(config_content) => {
                let config_value: serde_json::Value = serde_json::from_str(&config_content)?;
                if let Some(genesis_obj) = config_value.get("genesis") {
                    let genesis_config = qed_core::config::genesis::GenesisConfig::from_json(
                        &serde_json::to_string(genesis_obj)?
                    )?;
                    Some(genesis_config)
                } else {
                    None
                }
            }
            Err(e) => {
                warn!("Could not read config file {}: {}", cp_config.config_path, e);
                None
            }
        };

        let (deploy_contracts_root, user_tree_root) = if let Some(ref config) = genesis_config {
            let deploy_root = Self::process_genesis_precompiles(&qed_store, config).await?;
            let user_root = Self::process_genesis_contracts(&qed_store, config).await?;
            (deploy_root, user_root)
        } else {
            (QHashOut::ZERO, QHashOut::ZERO)
        };

        match qed_store.initialize_store(deploy_contracts_root, user_tree_root).await {
            Ok(checkpoint_id) if checkpoint_id == 0 => {
                qed_store.commit(0)?;
            }
            Ok(_) => {}
            Err(_) => {
                qed_store.rollback(0)?;
            }
        }

        let edge_command_queue = Arc::new(q.clone());

        let coord_config = CoordinatorConfig::get_standard(0);

        let qps = Arc::new(q.clone());

        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

        let coordinator_processor_ctx = CoordinatorProcessorContext::new(
            coord_config,
            Arc::new(qed_store.clone()),
            qps.clone(),
            qps.clone(),
            qps.clone(),
            qps.clone(),
            task_store.clone(),
            Arc::clone(&proof_verifier),
        )
        .await?;

        use qed_core::config::network_constants::get_default_worker_public_key;
        let coordinator_worker_circuits =
            QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library, get_default_worker_public_key::<F>());

        Ok(CoordinatorProcessNode::new(
            coordinator_processor_ctx,
            qed_store,
            edge_command_queue,
            q.clone(),
            q,
            proof_verifier,
            coordinator_worker_circuits,
            task_store,
        ))
    }

    pub async fn build_block(&mut self, slot: u64) -> anyhow::Result<u64> {
        let latest_l2_block_state = self.ctx.store.get_latest_l2_block_state().await?;
        let next_checkpoint_id = latest_l2_block_state.checkpoint_id + 1;

        // Check if there are pending tasks for this checkpoint
        if !self.ctx.has_pending_tasks(next_checkpoint_id).await
            .map_err(|e| anyhow::anyhow!("Failed to check pending tasks: {:?}", e))? {
            bail!(
                "No pending tasks for checkpoint {}, slot {}",
                next_checkpoint_id, slot
            );
        }

        let ctx = self.ctx.clone();
        let journal_store = self.journal_store.clone();
        self.retry_with_backoff(&format!("build block for checkpoint {}", next_checkpoint_id), || async {
            // Build and prove the block (all logic including logging is inside ctx.build_block)
            if let Err(e) = ctx.build_block(slot).await {
                journal_store.rollback(next_checkpoint_id)?;
                bail!("Failed to build and prove block: {}", e);
            }
            Ok(())
        }).await?;
        // Commit the changes
        self.journal_store.commit(next_checkpoint_id)?;
        self.ctx.commit_offset().await?;
        Ok(next_checkpoint_id)
    }

    async fn process_genesis_precompiles<SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>>(
        store: &SR,
        genesis_config: &GenesisConfig,
    ) -> anyhow::Result<QHashOut<F>> {
        use qed_prover::session::gen_contract_deploy_and_circuits_for_functions;
        use qedlang_core::dpn::vm::compile::QEDCompileResult;
        use qed_data::qdata::contract::{ContractCodeDefinition, ContractFunctionCodeDefinition};
        use qed_crypto::hash::traits::qhashable::QFieldHashable;
        use qed_data::config::store_config::QEDHasher;

        for precompile_path in genesis_config.get_precompile_paths() {
            let contract_path = std::path::Path::new(precompile_path);
            if !contract_path.exists() {
                warn!("Precompile contract file not found: {}", precompile_path);
                continue;
            }

            let contract_json = std::fs::read_to_string(contract_path)?;
            let function_defs: Vec<DPNFunctionCircuitDefinition> = serde_json::from_str(&contract_json)?;

            if !function_defs.is_empty() {
                let genesis_deployer = QHashOut::from_values(0, 0, 0, 1);

                let (circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions::<C, D>(
                    genesis_deployer,
                    MAX_CONTRACT_STATE_TREE_HEIGHT,
                    &function_defs,
                )?;

                let contract_id = store.get_latest_l2_block_state().await?.next_contract_id as u64;

                let function_tree_root = store.set_contract_function_whitelist_imm(
                    0,
                    contract_id,
                    &deploy_cmd.function_whitelist,
                ).await?;

                let contract_leaf = QEDContractLeaf {
                    deployer: deploy_cmd.deployer,
                    function_tree_root,
                    state_tree_height: F::from_canonical_u32(deploy_cmd.code_definition.state_tree_height as u32),
                };

                store.set_contract_leaf_data_imm(0, contract_id, &contract_leaf).await?;
                store.set_contract_code_definition_imm(0, contract_id, &deploy_cmd.code_definition).await?;

                let contract_leaf_hash = contract_leaf.qfhash::<QEDHasher>();
                store.set_contract_tree_leaf_hash_imm(0, contract_id, contract_leaf_hash).await?;
            }
        }

        Ok(store.get_contract_tree_root(0).await?)
    }

    async fn process_genesis_contracts<SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>>(
        store: &SR,
        genesis_config: &GenesisConfig,
    ) -> anyhow::Result<QHashOut<F>> {
        use qed_data::config::store_config::{UserTreeStore, QEDHasher};
        use qed_crypto::hash::merkle::utils::common::{QMerkleNode, SimpleMerkleNodeKey};
        use qed_crypto::hash::merkle::utils::simple_merkle_tree::SimpleMerkleTree;
        use qed_core::config::network_constants::{MAX_CONTRACT_STATE_TREE_HEIGHT, USERS_PER_REALM, REALM_USER_TREE_HEIGHT, COORDINATOR_USER_TREE_HEIGHT};
        use std::str::FromStr;
        use std::collections::HashMap;

        let mut realm_user_trees: HashMap<(u64, u64), SimpleMerkleTree<QEDHasher, QHashOut<F>>> = HashMap::new();

        for (contract_id_str, user_states) in genesis_config.get_all_contracts() {
            let contract_id = contract_id_str.parse::<u64>()?;

            for (user_id_str, user_state) in user_states {
                let user_id = user_id_str.parse::<u64>()?;
                let realm_id = user_id / USERS_PER_REALM;
                let user_index_in_realm = user_id % USERS_PER_REALM;

                let mut contract_state_tree = SimpleMerkleTree::<QEDHasher, QHashOut<F>>::new(MAX_CONTRACT_STATE_TREE_HEIGHT);

                for (slot_id_str, hex_value) in &user_state.slots {
                    let slot_id = slot_id_str.parse::<u64>()?;
                    let slot_value = QHashOut::<F>::from_str(hex_value)?;
                    let slot_key = SimpleMerkleNodeKey::new(MAX_CONTRACT_STATE_TREE_HEIGHT, slot_id);
                    contract_state_tree.set_node_value(slot_key, slot_value);
                }

                let contract_state_root = contract_state_tree.get_root();

                let realm_tree = realm_user_trees
                    .entry((contract_id, realm_id))
                    .or_insert_with(|| SimpleMerkleTree::<QEDHasher, QHashOut<F>>::new(REALM_USER_TREE_HEIGHT));

                let user_key = SimpleMerkleNodeKey::new(REALM_USER_TREE_HEIGHT, user_index_in_realm);
                realm_tree.set_node_value(user_key, contract_state_root);
            }
        }

        for ((contract_id, realm_id), realm_tree) in realm_user_trees {
            let realm_root = realm_tree.get_root();

            let coordinator_update = QMerkleNode {
                key: SimpleMerkleNodeKey {
                    level: COORDINATOR_USER_TREE_HEIGHT,
                    index: realm_id,
                },
                value: realm_root,
            };

            store.injest_user_tree_nodes_imm(0, 0, &[coordinator_update]).await?;
        }

        Ok(store.get_user_tree_root(0).await?)
    }
}

pub async fn run_processor(args: CoordinatorProcessorArgs) -> anyhow::Result<()> {
    let mut coordinator_processor = CoordinatorProcessNode::new_with_config(args).await?;
    let slot_timer= SlotTimer::new(LocalClock);
    let slot_timer_other = slot_timer.clone();
    loop {
        tokio::select! {
            is = coordinator_processor.wait_for_make_block() => {
                if !is {
                    continue;
                }
            }
            slot = slot_timer.wait_for_next_slot() => {
                info!("✅ Successfully wait for next slot: {}", slot);
            }
        }

        let slot = slot_timer_other.get_current_slot();
        match coordinator_processor.build_block(slot).await {
            Ok(checkpoint_id) => {
                info!(
                    "✅ Successfully built and committed block {}, slot {}",
                    checkpoint_id, slot
                );
            }
            Err(e) => {
                error!("❌ Failed to build block: {:?}, slot: {}", e, slot);
            }
        }
    }
}

impl<
    JL: Journal,
    SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
    DQ: CheckpointDrainQueueConsumerAsyncImmWithPosition,
    HQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm + NotificationQueue<CEQueueNotification>,
    WQ: WorkerEventTransmitterAsyncImm,
    PS: QProofStoreAsyncImm,
    ER: WorkerEventReceiverAsyncImm,
    TS: QProvingTaskStore,
> Retryable for CoordinatorProcessNode<JL, SR, DQ, HQ, WQ, PS, ER, TS>{}
