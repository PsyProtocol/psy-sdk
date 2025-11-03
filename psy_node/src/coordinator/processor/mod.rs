use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

use anyhow::{bail, Context};
use indexmap::IndexMap;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    hash::hash_types::RichField,
    plonk::config::PoseidonGoldilocksConfig,
};
use psy_config::{
    get_default_user_state_tree_root, get_default_worker_public_key,
    network_constants::{
        BATCH_USER_REGISTRAITION_SUB_TREE_HEIGHT, COORDINATOR_EDGE_TO_PROCESSOR_CHANNEL, COORDINATOR_USER_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT,
        GLOBAL_USER_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT, REALM_USER_TREE_HEIGHT, USERS_PER_REALM,
    },
};
use psy_common::{
    data::qhashout::QHashOut,
    job::{
        drain_queue::CheckpointDrainQueueConsumerAsyncImm,
        history_queue::{CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm},
        traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
        worker_queue::{WorkerEventReceiverAsyncImm, WorkerEventTransmitterAsyncImm},
    },
};
use psy_crypto::{
    common::{generic_circuit_verifier::GenericCircuitVerifier, user_id::get_user_id_from_registration_id},
    hash::{
        merkle::utils::{
            common::{QMerkleNode, SimpleMerkleNode, SimpleMerkleNodeKey},
            simple_merkle_tree::SimpleMerkleTree,
        },
        traits::qhashable::QFieldHashable,
    },
    signature::zk::data::ZKPublicKeyInfo,
};
use psy_data::{
    config::store_config::{PsyFelt, PsyHasher, UserTreeStore},
    qblock::cmds::deploy_contract::QBCDeployContractWithRoot,
    qdata::{
        contract::{ContractCodeDefinition, ContractFunctionCodeDefinition, PsyContractLeaf},
        realm_status::BasicRealmStatus,
        user::PsyUserLeaf,
        user_public_key::PsyUserPublicKeyRecord,
    },
    traits::qdatastore::qtreedata::{PsyComboDataStoreReaderWriterSync, QTreeDataStoreWriterSync},
};
use psy_network_circuit::coordinator::coordinator_helper::PsyCoordinatorCircuitManager;
use psy_prover::session::gen_contract_deploy_and_circuits_for_functions;
use psy_store::{
    node::coordinator::{InitializeParams, PsyCoordinatorStoreReaderAsync, PsyCoordinatorStoreWriterAsyncImm},
    queue::{
        new_redis_async_pool,
        redis_queue::{CheckpointDrainQueueConsumerAsyncImmWithPosition, NotificationQueue},
        rsmq_queue::CEQueueNotification,
        task_queue::{QProvingTaskStore, QProvingTaskStoreImpl},
        ProofStoreRedis,
    },
    store,
    store::{
        journal::{Journal, JournalStore},
        PsyStore,
    },
};
use psy_vm::dpn::vm::{compile::PsyCompileResult, def::DPNFunctionCircuitDefinition};
use serde_json;
use tokio::{
    sync::mpsc,
    time::{sleep_until, Instant},
};
use tracing::{debug, error, info, trace, warn};

use super::{
    args::CoordinatorProcessorArgs,
    backup::{try_backup_coordinator_checkpoint, CoordinatorS3BackupClient},
};
use crate::{
    common::{
        clock::SlotTimer,
        retry::Retryable,
        slot,
        slot::{LocalClock, Parity, Slot, SLOT_SIZE},
        verifier::get_cached_generic_verifier,
    },
    common_v2::traits::realm::BasicRealmStatusOnCoordinator,
    coordinator::state::processor::{CoordinatorConfig, CoordinatorProcessorContext},
    realm::RealmProcessor,
};

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = PsyFelt;

struct CoordinatorBackupRequest {
    checkpoint_id: u64,
    pair_to_set: Vec<(Vec<u8>, Vec<u8>)>,
    removed_keys: Vec<Vec<u8>>,
}

pub struct CoordinatorProcessNode<
    JL: Journal,
    SR: PsyCoordinatorStoreWriterAsyncImm<F> + PsyCoordinatorStoreReaderAsync<F> + Journal,
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
    pub coordinator_worker_circuits: PsyCoordinatorCircuitManager<C, D>,
    pub task_store: Arc<QProvingTaskStoreImpl>,
    pub backup_tx: Option<mpsc::UnboundedSender<CoordinatorBackupRequest>>,
}

impl<
        JL: Journal,
        SR: PsyCoordinatorStoreWriterAsyncImm<F> + PsyCoordinatorStoreReaderAsync<F> + Journal,
        DQ: CheckpointDrainQueueConsumerAsyncImmWithPosition,
        HQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm + NotificationQueue<CEQueueNotification>,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm,
        ER: WorkerEventReceiverAsyncImm,
        TS: QProvingTaskStore,
    > CoordinatorProcessNode<JL, SR, DQ, HQ, WQ, PS, ER, TS>
{
    pub async fn new(
        ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS, TS>,
        journal_store: JL,
        edge_command_queue: Arc<HQ>,
        proof_store: PS,
        event_receiver: ER,
        proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
        coordinator_worker_circuits: PsyCoordinatorCircuitManager<C, D>,
        task_store: Arc<QProvingTaskStoreImpl>,
    ) -> Self {
        let backup_tx = match CoordinatorS3BackupClient::new_from_env().await {
            Ok(client) => {
                info!("✅ S3 backup client initialized");
                let (tx, rx) = mpsc::unbounded_channel();
                tokio::spawn(async move {
                    CoordinatorProcessNode::backup_task(rx, client).await;
                });
                info!("Started coordinator backup task");

                Some(tx)
            }
            Err(e) => {
                warn!("⚠️ S3 backup client initialization failed: {}", e);
                None
            }
        };

        Self {
            ctx,
            journal_store,
            edge_command_queue,
            proof_store,
            event_receiver,
            proof_verifier,
            coordinator_worker_circuits,
            task_store,
            backup_tx,
        }
    }

    pub async fn wait_for_produce_block(&mut self) -> anyhow::Result<bool> {
        // Get current checkpoint to listen from
        let latest_block_state = self.ctx.store.get_latest_block_state().await?;
        let notify_message = self.edge_command_queue.consume_item(COORDINATOR_EDGE_TO_PROCESSOR_CHANNEL).await?;

        let latest_block_state = self.ctx.store.get_latest_block_state().await?;

        let CEQueueNotification::StartProduceBlock { next_checkpoint } = notify_message;
        debug!(
            "coordinator: wait_for_produce_block: next_checkpoint: {}, latest_block_state.checkpoint_id: {}",
            next_checkpoint, latest_block_state.checkpoint_id
        );

        match next_checkpoint.cmp(&latest_block_state.checkpoint_id) {
            std::cmp::Ordering::Equal => {
                info!("✅ Building new block for checkpoint {}", next_checkpoint);
                // No need to delete from history queue, it's already processed
                return Ok(false);
            }
            std::cmp::Ordering::Less => {
                warn!("⚠️ Outdated checkpoint {}, current {}", next_checkpoint, latest_block_state.checkpoint_id);
                // No need to delete from history queue, it's already processed
                return Ok(false);
            }
            std::cmp::Ordering::Greater if next_checkpoint - latest_block_state.checkpoint_id > 1 => {
                warn!(
                    "🚧 Future checkpoint {} too far ahead of {}",
                    next_checkpoint, latest_block_state.checkpoint_id
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
                trace!("✅ Successfully wait for produce block");
                true
            }
            Ok(false) => {
                info!("⚠️ No pending tasks, waiting for next checkpoint");
                tokio::time::sleep(Duration::from_millis(slot::SLOT_SIZE / 2)).await;
                false
            }
            Err(e) => {
                error!("❌ Error waiting for produce block: {:?}", e);
                tokio::time::sleep(Duration::from_millis(slot::SLOT_SIZE / 2)).await;
                false
            }
        }
    }
}

impl
    CoordinatorProcessNode<
        JournalStore<PsyStore>,
        JournalStore<PsyStore>,
        ProofStoreRedis,
        ProofStoreRedis,
        ProofStoreRedis,
        ProofStoreRedis,
        ProofStoreRedis,
        QProvingTaskStoreImpl,
    >
{
    pub async fn new_with_config(cp_config: CoordinatorProcessorArgs) -> anyhow::Result<Self> {
        let task_store =
            Arc::new(QProvingTaskStoreImpl::new(&cp_config.redis_uri, cp_config.redis_pool_size, &cp_config.queue_args.queue_biz_key).await?);
        let q = ProofStoreRedis::new(&cp_config.redis_uri, cp_config.queue_args.queue_biz_key.clone()).await?;

        let psy_store = store::from_backend(cp_config.backend.to_backend()).await?;
        let psy_store = JournalStore::new(psy_store);

        let config = psy_config::PsyConfigGoldilocks::from_file("config.json")?;
        let network = config.get_current_network()?;
        let genesis_config = network.genesis.clone();

        match Self::initialize_store(&psy_store, genesis_config).await {
            Ok(checkpoint_id) if checkpoint_id == 0 => {
                info!("Initialized store to genesis state");
                psy_store.commit(None)?;
            }
            Ok(checkpoint_id) => {
                info!("Store already initialized, current checkpoint {}", checkpoint_id);
            }
            Err(_) => {
                psy_store.rollback(0)?;
            }
        }

        let edge_command_queue = Arc::new(q.clone());

        let coord_config = CoordinatorConfig::get_standard();

        let qps = Arc::new(q.clone());

        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

        let coordinator_processor_ctx = CoordinatorProcessorContext::new(
            coord_config,
            Arc::new(psy_store.clone()),
            qps.clone(),
            qps.clone(),
            qps.clone(),
            qps.clone(),
            task_store.clone(),
            Arc::clone(&proof_verifier),
            cp_config.max_processed_contracts_per_block,
            cp_config.max_processed_users_per_block,
        )
        .await?;

        // get_default_worker_public_key is already imported at the top
        let coordinator_worker_circuits =
            PsyCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library, get_default_worker_public_key::<F>());

        Ok(CoordinatorProcessNode::new(
            coordinator_processor_ctx,
            psy_store,
            edge_command_queue,
            q.clone(),
            q,
            proof_verifier,
            coordinator_worker_circuits,
            task_store,
        )
        .await)
    }

    pub async fn initialize_store(
        psy_store: &JournalStore<PsyStore>,
        genesis_config: Option<psy_config::GenesisConfigGoldilocks>,
    ) -> anyhow::Result<u64> {
        let genesis_store_config = if let Some(ref config) = genesis_config {
            info!("initialize_store Some()");
            let deploy_root = Self::process_genesis_contracts(psy_store, config).await?;
            let register_users_root = Self::process_genesis_user_registrations(psy_store, config).await?;
            let user_root = Self::process_genesis_user_states(psy_store, config).await?;
            let next_contract_id = config.get_precompile_configs().len() as u32;
            let next_user_id = config.get_genesis_users().len() as u64;
            info!("next_user_id: {}", next_user_id);
            info!("next_contract_id: {}", next_contract_id);
            Some(InitializeParams {
                gutas_root: user_root,
                deploy_contracts_root: deploy_root,
                register_users_root,
                next_contract_id,
                next_user_id,
            })
        } else {
            info!("initialize_store none");
            None
        };
        psy_store.initialize_store(genesis_store_config).await
    }

    pub async fn build_block(&self, next_checkpoint_id: u64, slot: u64) -> anyhow::Result<u64> {
        let ctx = self.ctx.clone();
        let now = Instant::now();
        if let Err(e) = ctx.build_block(slot).await {
            ctx.rollback(next_checkpoint_id).await?;
            bail!("Rollback: Failed to build and prove block: {}", e);
        }
        let (pair_to_set, remove_keys) = match self.ctx.commit(next_checkpoint_id).await {
            Ok((pair_to_set, remove_keys)) => (pair_to_set, remove_keys),
            Err(e) => {
                ctx.rollback(next_checkpoint_id).await?;
                bail!("Rollback: Failed to commit block: {}", e);
            }
        };

        info!(
            "✅ Successfully built and committed block {}, slot {}, cost time: {:?}",
            next_checkpoint_id,
            slot,
            now.elapsed()
        );

        // Auto backup after successful commit
        if let Some(backup_tx) = &self.backup_tx {
            let pair_to_set = pair_to_set.into_iter().map(|pair| (pair.key, pair.value)).collect();
            let request = CoordinatorBackupRequest {
                checkpoint_id: next_checkpoint_id,
                pair_to_set,
                removed_keys: remove_keys,
            };
            if let Err(e) = backup_tx.send(request) {
                error!("❌ Failed to send backup request for checkpoint {}: {}", next_checkpoint_id, e);
            }
        }

        Ok(next_checkpoint_id)
    }

    pub async fn next_checkpoint_id(&self) -> anyhow::Result<u64> {
        let latest_block_state = self.ctx.store.get_latest_block_state().await?;
        Ok(latest_block_state.checkpoint_id + 1)
    }

    pub async fn has_pending_tasks(&self, checkpoint_id: u64) -> anyhow::Result<bool> {
        self.ctx.has_pending_tasks(checkpoint_id).await
    }

    async fn backup_task(mut rx: mpsc::UnboundedReceiver<CoordinatorBackupRequest>, backup_client: CoordinatorS3BackupClient) {
        info!("🚀 Coordinator backup task started");
        while let Some(request) = rx.recv().await {
            let CoordinatorBackupRequest {
                checkpoint_id,
                pair_to_set,
                removed_keys,
            } = request;
            // Retry up to 3 times with 1 second delay
            for retry_count in 0..=3 {
                match super::backup::create_checkpoint_backup(checkpoint_id, pair_to_set.clone(), removed_keys.clone()).await {
                    Ok(backup) => match backup_client.backup_checkpoint(&backup).await {
                        Ok(_) => {
                            info!("✅ Coordinator checkpoint {} backup succeeded", checkpoint_id);
                            break;
                        }
                        Err(e) if retry_count < 3 => {
                            warn!(
                                "⚠️ Coordinator backup retry {}/3 for checkpoint {}: {}",
                                retry_count + 1,
                                checkpoint_id,
                                e
                            );
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        }
                        Err(e) => {
                            error!("❌ Coordinator backup final failure for checkpoint {}: {}", checkpoint_id, e);
                        }
                    },
                    Err(e) if retry_count < 3 => {
                        warn!(
                            "⚠️ Coordinator backup creation retry {}/3 for checkpoint {}: {}",
                            retry_count + 1,
                            checkpoint_id,
                            e
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                    Err(e) => {
                        error!("❌ Failed to create coordinator backup for checkpoint {}: {}", checkpoint_id, e);
                        break;
                    }
                }
            }
        }
        warn!("🔚 Coordinator backup task stopped");
    }

    async fn backup_checkpoint(
        &self,
        backup_client: &CoordinatorS3BackupClient,
        checkpoint_id: u64,
        pair_to_set: Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>,
        removed_keys: Vec<Vec<u8>>,
    ) {
        let pair_to_set = pair_to_set.into_iter().map(|pair| (pair.key, pair.value)).collect();
        try_backup_coordinator_checkpoint(backup_client, checkpoint_id, pair_to_set, removed_keys).await;
    }

    async fn process_genesis_contracts<SR: PsyCoordinatorStoreWriterAsyncImm<F> + PsyCoordinatorStoreReaderAsync<F>>(
        store: &SR,
        genesis_config: &psy_config::GenesisConfigGoldilocks,
    ) -> anyhow::Result<QHashOut<F>> {
        for (contract_index, precompile_config) in genesis_config.get_precompile_configs().iter().enumerate() {
            // Convert bytecode JSON array to function definitions
            let function_defs: Vec<DPNFunctionCircuitDefinition> = precompile_config
                .bytecode
                .iter()
                .filter_map(|value| serde_json::from_value(value.clone()).ok())
                .collect();

            if !function_defs.is_empty() {
                let genesis_deployer: QHashOut<F> = precompile_config.deployer;

                let (circuits, deploy_cmd) =
                    gen_contract_deploy_and_circuits_for_functions::<C, D>(genesis_deployer, MAX_CONTRACT_STATE_TREE_HEIGHT, &function_defs)?;

                let contract_id = contract_index as u64;

                let function_tree_root = store
                    .set_contract_function_whitelist_imm(0, contract_id, &deploy_cmd.function_whitelist)
                    .await?;

                let contract_leaf = PsyContractLeaf {
                    deployer: deploy_cmd.deployer,
                    function_tree_root,
                    state_tree_height: F::from_canonical_u32(deploy_cmd.code_definition.state_tree_height as u32),
                };

                store.set_contract_leaf_data_imm(0, contract_id, &contract_leaf).await?;
                store
                    .set_contract_code_definition_imm(0, contract_id, &deploy_cmd.code_definition)
                    .await?;

                let contract_leaf_hash = contract_leaf.qfhash::<PsyHasher>();
                store.set_contract_tree_leaf_hash_imm(0, contract_id, contract_leaf_hash).await?;
            }
        }

        Ok(store.get_contract_tree_root(0).await?)
    }

    async fn process_genesis_user_states<
        SR: PsyCoordinatorStoreWriterAsyncImm<F> + PsyCoordinatorStoreReaderAsync<F> + QTreeDataStoreWriterSync<F>,
    >(
        store: &SR,
        genesis_config: &psy_config::GenesisConfigGoldilocks,
    ) -> anyhow::Result<QHashOut<F>> {
        let mut user_contract_states: HashMap<u64, HashMap<u64, QHashOut<F>>> = HashMap::new();
        let mut user_id_to_register_id: HashMap<u64, u64> = HashMap::new();

        for (contract_id, user_states) in genesis_config.get_all_contracts() {
            for (register_id, user_state) in user_states {
                let user_id = get_user_id_from_registration_id(*register_id);
                user_id_to_register_id.insert(user_id, *register_id);

                let mut contract_state_tree = SimpleMerkleTree::<PsyHasher, QHashOut<F>>::new(MAX_CONTRACT_STATE_TREE_HEIGHT);

                for (slot_id, slot_value) in &user_state.slots {
                    contract_state_tree.set_leaf(*slot_id, slot_value.clone());
                }

                let contract_state_root = contract_state_tree.get_root();

                user_contract_states
                    .entry(user_id)
                    .or_insert_with(HashMap::new)
                    .insert(*contract_id, contract_state_root);
            }
        }

        let mut realm_user_trees: HashMap<u64, SimpleMerkleTree<PsyHasher, QHashOut<F>>> = HashMap::new();

        let genesis_users = genesis_config.get_genesis_users();
        for (user_id, contract_states) in user_contract_states {
            let realm_id = user_id / USERS_PER_REALM;

            let mut user_contracts_tree = SimpleMerkleTree::<PsyHasher, QHashOut<F>>::new(GLOBAL_CONTRACT_TREE_HEIGHT);

            for (contract_id, contract_state_root) in &contract_states {
                user_contracts_tree.set_leaf(*contract_id, *contract_state_root);
            }

            let user_state_root = user_contracts_tree.get_root();

            let register_id = user_id_to_register_id[&user_id];
            let user_leaf = PsyUserLeaf {
                public_key: genesis_users[register_id as usize].qfhash::<PsyHasher>(),
                user_state_tree_root: user_state_root,
                balance: F::ZERO,
                nonce: F::ZERO,
                last_checkpoint_id: F::ZERO,
                event_index: F::ZERO,
                user_id: F::from_canonical_u64(user_id),
            };

            let user_leaf_hash = user_leaf.qfhash::<PsyHasher>();

            let realm_tree = realm_user_trees
                .entry(realm_id)
                .or_insert_with(|| SimpleMerkleTree::<PsyHasher, QHashOut<F>>::new(GLOBAL_USER_TREE_HEIGHT));

            realm_tree.set_leaf(user_id, user_leaf_hash);
        }

        let mut coordinator_updates = Vec::new();
        let mut realm_ids = Vec::new();
        let mut realm_statuses = Vec::new();

        for (realm_id, realm_tree) in realm_user_trees {
            let realm_root = realm_tree.get_node_value(&SimpleMerkleNodeKey {
                level: COORDINATOR_USER_TREE_HEIGHT,
                index: realm_id,
            });
            realm_ids.push(realm_id);
            realm_statuses.push(BasicRealmStatus {
                checkpoint_id: 0,
                realm_root_hash: realm_root,
            });

            let coordinator_update = QMerkleNode {
                key: SimpleMerkleNodeKey {
                    level: COORDINATOR_USER_TREE_HEIGHT,
                    index: realm_id,
                },
                value: realm_root,
            };
            coordinator_updates.push(coordinator_update);
        }

        if !coordinator_updates.is_empty() {
            store.injest_user_tree_nodes_imm(0, 0, &coordinator_updates).await?;
            store.set_realm_statuses(&realm_ids, &realm_statuses).await?;
        }

        let final_root = store.get_user_tree_root(0).await?;
        Ok(final_root)
    }

    async fn process_genesis_user_registrations<S: PsyCoordinatorStoreWriterAsyncImm<F> + PsyCoordinatorStoreReaderAsync<F>>(
        store: &S,
        config: &psy_config::GenesisConfigGoldilocks,
    ) -> anyhow::Result<QHashOut<F>> {
        let start_registration_user_id = 0u64;
        let user_registrations = config.get_genesis_users();

        let new_user_records = user_registrations
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let registration_id = start_registration_user_id + (i as u64);
                let user_id = get_user_id_from_registration_id(registration_id);
                PsyUserPublicKeyRecord {
                    public_key_param: x.public_key_param,
                    fingerprint: x.fingerprint,
                    public_key: x.qfhash::<PsyHasher>(),
                    user_id,
                    checkpoint_id: 0,
                }
            })
            .collect::<Vec<_>>();

        store.set_user_public_key_records(&new_user_records).await?;

        let new_public_keys: Vec<QHashOut<F>> = user_registrations.iter().map(|x| x.qfhash::<PsyHasher>()).collect();

        let _wits = store
            .batch_append_user_registration_tree_imm(
                0,
                start_registration_user_id,
                BATCH_USER_REGISTRAITION_SUB_TREE_HEIGHT as u8,
                &new_public_keys,
            )
            .await?;

        Ok(store.get_user_registration_tree_root(0).await?)
    }
}

pub async fn run_processor(args: CoordinatorProcessorArgs) -> anyhow::Result<()> {
    let mut coordinator_processor = CoordinatorProcessNode::new_with_config(args).await?;
    let slot_timer = SlotTimer::new(LocalClock);
    let slot_timer_other = slot_timer.clone();
    loop {
        let next_checkpoint_id = coordinator_processor.next_checkpoint_id().await?;
        tokio::select! {
            biased;
            slot = slot_timer.wait_for_next_slot() => {
                // if slot.is_odd() {
                //     continue;
                // }
                trace!("✅ Successfully wait for next slot: {}", slot);
                if !coordinator_processor.has_pending_tasks(next_checkpoint_id).await? {
                    trace!("⚠️ No pending tasks for checkpoint {}, waiting for next checkpoint", next_checkpoint_id);
                    continue;
                }
            }
            is = coordinator_processor.wait_for_make_block() => {
                if !is {
                    continue;
                }
                debug!("✅ make block, checkpoint {}", next_checkpoint_id);
            }
        }

        let slot = slot_timer_other.get_current_slot();
        match coordinator_processor.build_block(next_checkpoint_id, slot).await {
            Ok(checkpoint_id) => {
                info!("✅ Successfully built block for checkpoint {}, slot {}", checkpoint_id, slot);
            }
            Err(err) => {
                error!(
                    "❌Failed to build block checkpoint: {}, error: {:?}, slot: {}",
                    next_checkpoint_id, err, slot
                );
            }
        }
    }
}

impl<
        JL: Journal,
        SR: PsyCoordinatorStoreWriterAsyncImm<F> + PsyCoordinatorStoreReaderAsync<F> + Journal,
        DQ: CheckpointDrainQueueConsumerAsyncImmWithPosition,
        HQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm + NotificationQueue<CEQueueNotification>,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm,
        ER: WorkerEventReceiverAsyncImm,
        TS: QProvingTaskStore,
    > Retryable for CoordinatorProcessNode<JL, SR, DQ, HQ, WQ, PS, ER, TS>
{
}
