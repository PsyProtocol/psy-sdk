use std::{
    marker::Sync,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::ensure;
use chrono::Utc;
use kvq::traits::KVQPair;
use plonky2::{
    field::types::Field,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use psy_common::{
    data::qhashout::QHashOut,
    job::{
        drain_queue::{CheckpointDrainQueueConsumerAsyncImm, WithDrainQueueMetadata},
        history_queue::CheckpointHistoryQueueEmitterAsyncImm,
        id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID, QProvingTask, QProvingTaskGraph},
        traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
        worker_queue::WorkerEventTransmitterAsyncImm,
    },
    utils::graph::BidirectionalGraph,
};
use psy_config::network_constants::{
    BATCH_DEPLOY_CONTRACT_SUB_TREE_HEIGHT, BATCH_USER_REGISTRAITION_MAX_SUB_TREES, BATCH_USER_REGISTRAITION_SUB_TREE_HEIGHT,
    COORDINATOR_USER_TREE_HEIGHT, COORD_API_DEPLOY_CONTRACT_CHANNEL_ID, COORD_API_GUTA_FROM_REALMS_CHANNEL_ID, COORD_API_REGISTER_USER_CHANNEL_ID,
    CST_USER_UPDATE_CHANNEL_ID, DA_CHALLENGE_WINDOW, REALM_USER_TREE_HEIGHT,
};
use psy_crypto::{
    common::{
        cached_circuit_library::get_cached_circuit_library, circuit_library::CircuitInfoLibraryCore,
        generic_circuit_verifier::GenericCircuitVerifier, user_id::get_user_id_from_registration_id,
    },
    hash::{
        merkle::{
            core::compute_historical_and_current_merkle_roots_core_gt,
            treeprover::{
                data::CircuitInputWithDependencies, subtree::SubTreeNodeStateTransition, tree_helper::plan_tree_prover_from_leaves,
                AggStateTransition, AggStateTransitionInput, AggWTLeafAggregator,
            },
            utils::common::{SimpleMerkleNode, SimpleMerkleNodeKey},
        },
        traits::{
            hasher::{FieldQHasher, MerkleZeroHasher},
            qhashable::QFieldHashable,
        },
    },
    signature::zk::data::ZKPublicKeyInfo,
};
use psy_data::{
    config::store_config::{PsyFelt, PsyHasher, QCheckpointSyncInfoCompact, UserPublicKeyTableStore, UserTreeStore},
    guta::{
        api::SubmitGUTARealmResultAPIQueueItem,
        header::GlobalUserTreeAggregatorHeader,
        proof_input::{
            GUTANoChangeFullInput, VerifyGUTAToCapCircuitInputSimple, VerifyGUTAToCapUpgradeCheckpointCircuitInputSimple,
            VerifyTwoGUTAProofGadgetStandardInputSimple, VerifyTwoGUTAProofUpgradeCheckpointStandardInputSimple,
        },
        stats::GUTAStats,
    },
    models::{checkpoint::user_public_keys::PsyUserPublicKeyHelperModelCore, kvq_merkle::model::KVQFixedConfigMerkleTreeModelReaderCore},
    proof_store::builder::ProofStoreBuilder,
    protocol::circuit_inputs::{
        agg_part_1::QCAggUserRegistartionDeployContractsGUTAInput,
        checkpoint_transition::{QCPsyCheckpointStateTransitionInput, QCPsyCheckpointStateTransitionInputPartial},
    },
    qblock::cmds::deploy_contract::QBCDeployContractWithRoot,
    qdata::{
        checkpoint::{
            CheckpointSyncInfo, PsyBlockState, PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf, PsyCheckpointLeafCompactWithStateRoots,
            PsyCheckpointLeafStats,
        },
        contract::PsyContractLeaf,
        contract_metadata::ContractMetaData,
        contract_uuid::ContractUUID,
        pm_jobs_completed_stats::PMJobsCompletedStats,
        pm_reward_commitment::PMRewardCommitment,
        realm_status::BasicRealmStatus,
        user_public_key::PsyUserPublicKeyRecord,
    },
};
use psy_network_circuit::guta::gadgets::guta_header;
use psy_store::{
    node::coordinator::{PsyCoordinatorStoreReaderAsync, PsyCoordinatorStoreWriterAsyncImm},
    queue::{
        redis_queue::{CheckpointDrainQueueConsumerAsyncImmWithPosition, QueueOffsetState, MAX_CHECKPOINT_COUNT},
        task_queue::QProvingTaskStore,
    },
    store::{
        journal::{Journal, JournalStore},
        PsyStore,
    },
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, trace, warn};

use crate::common::slot::SLOT_SIZE;

type F = PsyFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CoordinatorConfig {
    pub users_per_realm: usize,
    pub realm_root_level: u8,
    pub coordinator_id: u32,

    pub guta_channel_id: u64,
    pub register_user_channel_id: u64,
    pub deploy_contract_channel_id: u64,

    pub register_user_tree_batch_height: u8,
    pub register_user_tree_batch_size: usize,

    pub deploy_contracts_tree_batch_height: u8,

    pub register_users_circuit_whitelist: QHashOut<F>,
    pub register_user_dummy_state_root: QHashOut<F>,

    pub deploy_contracts_circuit_whitelist: QHashOut<F>,
    pub deploy_contracts_dummy_state_root: QHashOut<F>,

    pub guta_circuit_whitelist: QHashOut<F>,
}
impl CoordinatorConfig {
    pub fn get_standard() -> Self {
        let library = get_cached_circuit_library::<F>();

        let realm_root_level = COORDINATOR_USER_TREE_HEIGHT;
        let users_per_realm = 1usize << (REALM_USER_TREE_HEIGHT as usize);

        Self {
            users_per_realm,
            realm_root_level,
            coordinator_id: 1u32 << COORDINATOR_USER_TREE_HEIGHT,
            guta_channel_id: COORD_API_GUTA_FROM_REALMS_CHANNEL_ID,
            register_user_channel_id: COORD_API_REGISTER_USER_CHANNEL_ID,
            deploy_contract_channel_id: COORD_API_DEPLOY_CONTRACT_CHANNEL_ID,
            register_user_tree_batch_height: BATCH_USER_REGISTRAITION_SUB_TREE_HEIGHT as u8,
            register_user_tree_batch_size: BATCH_USER_REGISTRAITION_MAX_SUB_TREES,
            deploy_contracts_tree_batch_height: BATCH_DEPLOY_CONTRACT_SUB_TREE_HEIGHT as u8,
            register_users_circuit_whitelist: library
                .get_agg_whitelist::<PsyHasher>(ProvingJobCircuitType::AppendUserRegistrationTree)
                .unwrap(),
            register_user_dummy_state_root: QHashOut::ZERO,
            deploy_contracts_circuit_whitelist: library
                .get_agg_whitelist::<PsyHasher>(ProvingJobCircuitType::BatchDeployContracts)
                .unwrap(),
            deploy_contracts_dummy_state_root: QHashOut::ZERO,
            guta_circuit_whitelist: library
                .get_group_inclusion_proof(ProvingJobCircuitType::GUTATwoGUTA, ProvingJobCircuitType::GUTATwoGUTA)
                .unwrap()
                .root,
        }
    }
}
#[derive(Clone)]
pub struct CoordinatorProcessorContext<
    SR: PsyCoordinatorStoreWriterAsyncImm<F> + PsyCoordinatorStoreReaderAsync<F> + Journal,
    DQ: CheckpointDrainQueueConsumerAsyncImm + CheckpointDrainQueueConsumerAsyncImmWithPosition,
    HQ: CheckpointHistoryQueueEmitterAsyncImm,
    WQ: WorkerEventTransmitterAsyncImm,
    PS: QProofStoreAsyncImm + QProofStoreWriterAsyncImm + QProofStoreReaderAsync,
    TS: QProvingTaskStore,
> {
    pub store: Arc<SR>,
    pub checkpoint_queue: Arc<DQ>,
    pub sync_queue: Arc<HQ>,
    pub prover_queue: Arc<WQ>,
    pub proof_store: Arc<PS>,
    pub task_store: Arc<TS>,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub coordinator_config: CoordinatorConfig,
    pub max_processed_contracts_per_block: Option<isize>,
    pub max_processed_users_per_block: Option<isize>,
}

impl<
        SR: PsyCoordinatorStoreWriterAsyncImm<F> + PsyCoordinatorStoreReaderAsync<F> + Journal,
        DQ: CheckpointDrainQueueConsumerAsyncImm + CheckpointDrainQueueConsumerAsyncImmWithPosition,
        HQ: CheckpointHistoryQueueEmitterAsyncImm,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm + Sync,
        TS: QProvingTaskStore + Sync,
    > CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS, TS>
{
    pub async fn new(
        coordinator_config: CoordinatorConfig,
        store: Arc<SR>,
        checkpoint_queue: Arc<DQ>,
        sync_queue: Arc<HQ>,
        prover_queue: Arc<WQ>,
        proof_store: Arc<PS>,
        task_store: Arc<TS>,
        proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
        max_processed_contracts_per_block: Option<isize>,
        max_processed_users_per_block: Option<isize>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            coordinator_config,
            store,
            checkpoint_queue,
            prover_queue,
            sync_queue,
            proof_store,
            task_store,
            proof_verifier,
            max_processed_contracts_per_block,
            max_processed_users_per_block,
        })
    }

    pub fn verify_proof_of_type(&self, circuit_type: ProvingJobCircuitType, proof: &ProofWithPublicInputs<F, C, D>) -> anyhow::Result<()> {
        self.proof_verifier.verify_proof_of_type(circuit_type, proof)
    }

    pub async fn handle_deploy_contracts(
        &self,
        checkpoint_id: u64,
        slot_id: u64,
    ) -> anyhow::Result<(
        Vec<Vec<QProvingJobDataID>>,
        AggStateTransition<F>,
        u32,
        BidirectionalGraph<QProvingJobDataID>,
        QueueOffsetState,
    )> {
        let last_blockstate = self.store.get_latest_block_state().await?;

        let last_contract_tree_root = self.store.get_contract_tree_root(checkpoint_id).await?;
        tracing::debug!("last_contract_tree_root: {}", last_contract_tree_root);
        let (deploy_contract_items, consumption_state) = self
            .checkpoint_queue
            .peek_with_position::<WithDrainQueueMetadata<QBCDeployContractWithRoot<F>>>(
                self.max_processed_contracts_per_block,
                self.coordinator_config.deploy_contract_channel_id,
                checkpoint_id,
            )
            .await?;

        let new_contract_leaves = deploy_contract_items
            .iter()
            .map(|x| PsyContractLeaf {
                deployer: x.payload.deployer,
                function_tree_root: x.payload.function_whitelist_root,
                state_tree_height: F::from_canonical_u16(x.payload.code_definition.state_tree_height),
            })
            .collect::<Vec<_>>();

        let new_hashes = new_contract_leaves.iter().map(|x| x.qfhash::<PsyHasher>()).collect::<Vec<_>>();

        let start_contract_id = last_blockstate.next_contract_id;

        let now = Instant::now();
        for (i, dc) in deploy_contract_items.iter().enumerate() {
            let contract_id = start_contract_id as u64 + i as u64;
            self.store
                .set_contract_code_definition_imm(checkpoint_id, contract_id, &dc.payload.code_definition)
                .await?;
            self.store
                .set_contract_function_whitelist_imm(checkpoint_id, contract_id, &dc.payload.function_whitelist)
                .await?;

            let contract_uuid = ContractUUID {
                checkpoint_id: dc.metadata.checkpoint_id,
                uuid: dc.metadata.item_id,
            };
            let contract_metadata = ContractMetaData {
                checkpoint_id: dc.metadata.checkpoint_id,
                contract_id,
                deployer: dc.payload.deployer,
                function_whitelist_root: dc.payload.function_whitelist_root,
            };
            self.store.set_contract_metadata(contract_uuid, &contract_metadata).await?;
        }
        for (i, l) in new_contract_leaves.iter().enumerate() {
            self.store
                .set_contract_leaf_data_imm(checkpoint_id, start_contract_id as u64 + i as u64, l)
                .await?;
        }
        tracing::debug!(
            "deploy contract cost time: {:?}, deploy_contract_items len: {}",
            now.elapsed(),
            deploy_contract_items.len()
        );
        let next_contract_id = start_contract_id + new_contract_leaves.len() as u32;
        let mut psb = ProofStoreBuilder::new();
        let now = Instant::now();
        let (start_indexes, spiderman_append_proofs) = self
            .store
            .batch_append_contract_tree_imm(
                checkpoint_id,
                start_contract_id as u64,
                self.coordinator_config.deploy_contracts_tree_batch_height,
                &new_hashes,
            )
            .await?;
        tracing::debug!("batch_append_contract_tree_imm cost time: {:?}", now.elapsed());
        tracing::debug!(
            "deploy contracts spiderman proofs: {}",
            serde_json::to_string_pretty(&spiderman_append_proofs).unwrap()
        );
        let wits = spiderman_append_proofs
            .into_iter()
            .zip(start_indexes)
            .enumerate()
            .map(|(i, (spiderman_proof, start_idx))| {
                let leaves_per_subtree = 1 << self.coordinator_config.deploy_contracts_tree_batch_height;
                let zero_hash = PsyHasher::get_zero_hash(0);
                let mut contract_leaves = Vec::with_capacity(leaves_per_subtree);

                let mut new_contract_idx = start_idx;

                for j in 0..spiderman_proof.web_proof_new_leaves.len() {
                    let new_leaf_hash = spiderman_proof.web_proof_new_leaves[j];
                    let old_leaf_hash = spiderman_proof.web_proof_old_leaves[j];

                    let is_added = old_leaf_hash == zero_hash && new_leaf_hash != zero_hash;

                    if is_added && new_contract_idx < new_contract_leaves.len() {
                        contract_leaves.push(new_contract_leaves[new_contract_idx]);
                        new_contract_idx += 1;
                    } else {
                        contract_leaves.push(PsyContractLeaf::default());
                    }
                }

                while contract_leaves.len() < leaves_per_subtree {
                    contract_leaves.push(PsyContractLeaf::default());
                }

                self.push_deploy_contracts_request(
                    checkpoint_id,
                    slot_id,
                    self.coordinator_config.coordinator_id,
                    i as u32,
                    &mut psb,
                    spiderman_proof,
                    contract_leaves,
                )
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let (batch_deploy_contract_tree_job_ids, root_transition_batch_deploy_contract_tree, batch_deploy_contract_graph) =
            plan_tree_prover_from_leaves::<F, ProofStoreBuilder, AggWTLeafAggregator, _, AggStateTransitionInput<F>>(
                &wits,
                &mut psb,
                QProvingJobDataID::new_proof_job_id(
                    checkpoint_id,
                    slot_id,
                    self.coordinator_config.coordinator_id,
                    ProvingJobCircuitType::DummyBatchDeployContractsAggregate,
                    0,
                    0,
                ),
                last_contract_tree_root,
                self.coordinator_config.deploy_contracts_circuit_whitelist,
            )?;
        tracing::debug!(
            "root_transition_batch_deploy_contract_tree: {}",
            serde_json::to_string_pretty(&root_transition_batch_deploy_contract_tree).unwrap()
        );

        self.proof_store.set_bytes_by_id_batch(&psb.kvs).await?;
        let new_contract_tree_root = self.store.get_contract_tree_root(checkpoint_id).await?;
        tracing::debug!("new_contract_tree_root: {}", new_contract_tree_root);

        Ok((
            batch_deploy_contract_tree_job_ids,
            AggStateTransition {
                state_transition_start: last_contract_tree_root,
                state_transition_end: new_contract_tree_root,
            },
            next_contract_id,
            batch_deploy_contract_graph,
            consumption_state,
        ))
    }

    pub async fn handle_user_registrations(
        &self,
        checkpoint_id: u64,
        slot_id: u64,
    ) -> anyhow::Result<(
        Vec<Vec<QProvingJobDataID>>,
        AggStateTransition<F>,
        Vec<ZKPublicKeyInfo<F>>,
        Vec<QHashOut<F>>,
        BidirectionalGraph<QProvingJobDataID>,
        QueueOffsetState,
    )> {
        let last_blockstate = self.store.get_latest_block_state().await?;

        let last_user_registration_tree_root = self.store.get_user_registration_tree_root(checkpoint_id).await?;
        tracing::debug!("last_user_registration_tree_root: {}", last_user_registration_tree_root);
        let (user_registrations, consumption_state) = self
            .checkpoint_queue
            .peek_with_position::<ZKPublicKeyInfo<F>>(self.max_processed_users_per_block, COORD_API_REGISTER_USER_CHANNEL_ID, checkpoint_id)
            .await?;

        let start_registration_user_id = last_blockstate.next_user_id;

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
                    checkpoint_id,
                }
            })
            .collect::<Vec<_>>();
        tracing::info!(
            "injest_checkpoint_sync_data_imm: start_registration_user_id: {}, new_user_records len: {:?}",
            start_registration_user_id,
            new_user_records.len(),
        );
        self.store.set_user_public_key_records(&new_user_records).await?;

        let new_public_keys = user_registrations.iter().map(|x| x.to_hash::<PsyHasher>()).collect::<Vec<_>>();

        let mut psb = ProofStoreBuilder::new();
        let now = Instant::now();
        let res = self
            .store
            .batch_append_user_registration_tree_imm(
                checkpoint_id,
                start_registration_user_id,
                self.coordinator_config.register_user_tree_batch_height,
                &new_public_keys,
            )
            .await?;
        tracing::debug!("batch_append_user_registration_tree_imm cost time: {:?}", now.elapsed());
        tracing::debug!("user registrations spiderman proofs: {}", serde_json::to_string_pretty(&res).unwrap());
        let (start_indexes, spiderman_proofs) = res;
        let wits = spiderman_proofs
            .chunks(self.coordinator_config.register_user_tree_batch_size)
            .enumerate()
            .map(|(i, c)| {
                self.push_user_registration_request(
                    checkpoint_id,
                    slot_id,
                    self.coordinator_config.coordinator_id,
                    i as u32,
                    &mut psb,
                    c.to_vec(),
                )
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let siblings = if wits.len() != 0 {
            self.store
                .get_user_registration_tree_merkle_proof(checkpoint_id, start_registration_user_id)
                .await?
                .siblings
        } else {
            Vec::new()
        };

        let (append_user_registration_tree_job_ids, root_transition_append_user_registration_tree, append_user_registration_graph) =
            plan_tree_prover_from_leaves::<F, ProofStoreBuilder, AggWTLeafAggregator, _, AggStateTransitionInput<F>>(
                &wits,
                &mut psb,
                QProvingJobDataID::new_proof_job_id(
                    checkpoint_id,
                    slot_id,
                    self.coordinator_config.coordinator_id,
                    ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate,
                    0,
                    0,
                ),
                last_user_registration_tree_root,
                self.coordinator_config.register_users_circuit_whitelist,
            )?;
        tracing::debug!(
            "root_transition_append_user_registration_tree: {}",
            serde_json::to_string_pretty(&root_transition_append_user_registration_tree).unwrap()
        );

        self.proof_store.set_bytes_by_id_batch(&psb.kvs).await?;
        let new_user_registration_tree_root = self.store.get_user_registration_tree_root(checkpoint_id).await?;
        tracing::debug!("new_user_registration_tree_root: {}", new_user_registration_tree_root);

        Ok((
            append_user_registration_tree_job_ids,
            AggStateTransition {
                state_transition_start: last_user_registration_tree_root,
                state_transition_end: new_user_registration_tree_root,
            },
            user_registrations,
            siblings,
            append_user_registration_graph,
            consumption_state,
        ))
    }

    pub async fn handle_guta_from_realms(
        &self,
        checkpoint_id: u64,
        slot_id: u64,
    ) -> anyhow::Result<(
        Vec<Vec<QProvingJobDataID>>,
        GlobalUserTreeAggregatorHeader<F>,
        BidirectionalGraph<QProvingJobDataID>,
        Vec<QueueOffsetState>,
    )> {
        tracing::debug!(checkpoint_id = checkpoint_id, "Processing checkpoint");
        let mut offset_states = vec![];
        let (mut guta_queue_items, consumption_state) = self
            .checkpoint_queue
            .peek_with_position::<SubmitGUTARealmResultAPIQueueItem<F>>(None, self.coordinator_config.guta_channel_id, checkpoint_id)
            .await?;
        offset_states.push(consumption_state);
        tracing::debug!(guta_queue_items = %serde_json::to_string_pretty(&guta_queue_items).unwrap(), "GUTA queue items");
        let last_checkpoint_id = checkpoint_id.saturating_sub(1);
        let last_checkpoint_tree_root = self.store.get_checkpoint_tree_root(last_checkpoint_id).await?;
        tracing::debug!("last_checkpoint_tree_root: {}", last_checkpoint_tree_root);
        //let mut guta_queue_items = guta_queue_items.into_iter().filter(|x| {
        //     let is = x.checkpoint_id <= checkpoint_id && x.checkpoint_id >=
        // checkpoint_id.saturating_sub(2);//todo: fix this     if !is {
        //         warn!("Filtering GUTA queue items for checkpoint_id: {}, current guta
        // checkpoint_id: {}", checkpoint_id, x.checkpoint_id);     }
        //     is
        // }).collect::<Vec<_>>();

        let (realm_ids, realm_statuses): (Vec<u64>, Vec<BasicRealmStatus<F>>) = guta_queue_items
            .iter()
            .map(|x| {
                (
                    x.realm_id,
                    BasicRealmStatus {
                        checkpoint_id,
                        realm_root_hash: x.top_line_proof.new_root,
                    },
                )
            })
            .unzip();
        self.store.set_realm_statuses(&realm_ids, &realm_statuses).await?;

        if guta_queue_items.len() == 0 {
            tracing::debug!("No GUTA queue items");
            let roots = self.store.get_checkpoint_global_state_roots(last_checkpoint_id).await?;
            let checkpoint_tree_proof = self.store.get_checkpoint_tree_merkle_proof(checkpoint_id, last_checkpoint_id).await?;
            let checkpoint_leaf = self.store.get_checkpoint_leaf_data(last_checkpoint_id).await?;
            let guta_header = GlobalUserTreeAggregatorHeader {
                guta_circuit_whitelist: self.coordinator_config.guta_circuit_whitelist,
                checkpoint_tree_root: checkpoint_tree_proof.root,
                state_transition: SubTreeNodeStateTransition {
                    old_node_value: roots.user_tree_root,
                    new_node_value: roots.user_tree_root,
                    node_index: F::ZERO,
                    node_level: F::ZERO,
                },
                stats: GUTAStats {
                    fees_collected: F::ZERO,
                    user_ops_processed: F::ZERO,
                    total_transactions: F::ZERO,
                    slots_modified: F::ZERO,
                },
            };
            tracing::debug!(guta_header = %serde_json::to_string_pretty(&guta_header).unwrap(), guta_header_hash = %guta_header.qfhash::<PsyHasher>(), "GUTA header");
            let input = GUTANoChangeFullInput {
                checkpoint_tree_proof,
                checkpoint_leaf: PsyCheckpointLeafCompactWithStateRoots {
                    checkpoint_leaf: checkpoint_leaf.to_compact::<PsyHasher>(),
                    global_state_roots: roots,
                },
            };
            tracing::debug!(input = %serde_json::to_string_pretty(&input).unwrap(), "Single GUTA input");

            let id = QProvingJobDataID::new(
                QJobTopic::GenerateStandardProof,
                checkpoint_id,
                slot_id,
                self.coordinator_config.coordinator_id,
                0,
                0,
                ProvingJobCircuitType::GUTANoChange,
                ProvingJobDataType::InputWitness,
                0,
            );

            self.proof_store
                .set_bytes_by_id(id.get_input_witness_id(), &bincode::serialize(&input)?)
                .await?;

            let mut graph = BidirectionalGraph::new();
            graph.add_node(id.get_output_id());
            return Ok((vec![vec![id]], guta_header, graph, offset_states));
        } else if guta_queue_items.len() == 1 {
            tracing::debug!("Processing single GUTA queue item");
            let old_mp = self
                .store
                .get_user_top_tree_merkle_proof(checkpoint_id, self.coordinator_config.realm_root_level, guta_queue_items[0].realm_id)
                .await?;

            // todo: check old_mp.value == guta_queue_items[0].top_line_proof.old_root

            if guta_queue_items[0].checkpoint_tree_root != last_checkpoint_tree_root {
                tracing::warn!("Checkpoint tree root in GUTA queue item does not match last checkpoint tree root");
                let real_guta_checkpoint_id = guta_queue_items[0].checkpoint_id.saturating_sub(1);
                let historical_checkpoint_proof = self
                    .store
                    .get_checkpoint_tree_merkle_proof(last_checkpoint_id, real_guta_checkpoint_id)
                    .await?;
                ensure!(historical_checkpoint_proof.root == last_checkpoint_tree_root);
                let (historical_root, current_root) =
                    compute_historical_and_current_merkle_roots_core_gt::<QHashOut<F>, PsyHasher>(&historical_checkpoint_proof);
                ensure!(current_root == historical_checkpoint_proof.root);
                ensure!(current_root == last_checkpoint_tree_root);
                ensure!(historical_root == guta_queue_items[0].checkpoint_tree_root);
                let rw = VerifyGUTAToCapUpgradeCheckpointCircuitInputSimple {
                    historical_checkpoint_proof,
                    guta_proof_header: GlobalUserTreeAggregatorHeader {
                        guta_circuit_whitelist: self.coordinator_config.guta_circuit_whitelist,
                        checkpoint_tree_root: guta_queue_items[0].checkpoint_tree_root,
                        state_transition: SubTreeNodeStateTransition {
                            old_node_value: guta_queue_items[0].top_line_proof.old_root,
                            new_node_value: guta_queue_items[0].top_line_proof.new_root,
                            node_index: F::from_noncanonical_u64(guta_queue_items[0].top_line_proof.index),
                            node_level: F::from_canonical_u64(
                                (self.coordinator_config.realm_root_level as usize + guta_queue_items[0].top_line_proof.siblings.len()) as u64,
                            ),
                        },
                        stats: guta_queue_items[0].guta_stats,
                    },
                    top_line_siblings: [guta_queue_items[0].top_line_proof.siblings.clone(), old_mp.siblings].concat(),
                };
                tracing::debug!(rw = %serde_json::to_string_pretty(&rw).unwrap(), "Register witness");
                let r_with_deps = CircuitInputWithDependencies {
                    input: rw,
                    dependencies: vec![guta_queue_items[0].proof_id],
                };
                tracing::debug!(guta_item = %serde_json::to_string_pretty(&guta_queue_items[0]).unwrap(), "First GUTA queue item");

                let id = QProvingJobDataID::new(
                    QJobTopic::GenerateStandardProof,
                    checkpoint_id,
                    slot_id,
                    self.coordinator_config.coordinator_id,
                    0,
                    0,
                    ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade,
                    ProvingJobDataType::InputWitness,
                    0,
                );

                let new_nodes = guta_queue_items
                    .iter()
                    .map(|x| {
                        assert_eq!(x.top_line_proof.index, x.realm_id, "right now guta proofs with top line are not allowed");
                        SimpleMerkleNode {
                            key: SimpleMerkleNodeKey {
                                level: self.coordinator_config.realm_root_level,
                                index: x.realm_id,
                            },
                            value: x.top_line_proof.new_root,
                        }
                    })
                    .collect::<Vec<_>>();

                let mut res = self.store.injest_user_tree_nodes_imm(checkpoint_id, 0, &new_nodes).await?;
                tracing::debug!(res = %serde_json::to_string_pretty(&res).unwrap(), "GUTA result");
                let good_old = self
                    .store
                    .get_user_top_tree_cap_root(checkpoint_id - 1, res.nearest_common_ancestor_level, res.nearest_common_ancestor_index)
                    .await?;
                tracing::debug!(good_old = %good_old, "Good old value");

                res.link_proof.old_value = good_old;

                self.proof_store
                    .set_bytes_by_id(id.get_input_witness_id(), &bincode::serialize(&r_with_deps)?)
                    .await?;

                let mut graph = BidirectionalGraph::new();
                graph.add_node(id.get_output_id());
                for dep in &r_with_deps.dependencies {
                    graph.add_edge(id.get_output_id(), *dep);
                }
                return Ok((vec![vec![id]], r_with_deps.input.get_new_guta_header::<PsyHasher>(), graph, offset_states));
            }

            let rw = VerifyGUTAToCapCircuitInputSimple {
                guta_proof_header: GlobalUserTreeAggregatorHeader {
                    guta_circuit_whitelist: self.coordinator_config.guta_circuit_whitelist,
                    checkpoint_tree_root: guta_queue_items[0].checkpoint_tree_root,
                    state_transition: SubTreeNodeStateTransition {
                        old_node_value: guta_queue_items[0].top_line_proof.old_root,
                        new_node_value: guta_queue_items[0].top_line_proof.new_root,
                        node_index: F::from_noncanonical_u64(guta_queue_items[0].top_line_proof.index),
                        node_level: F::from_canonical_u64(
                            (self.coordinator_config.realm_root_level as usize + guta_queue_items[0].top_line_proof.siblings.len()) as u64,
                        ),
                    },
                    stats: guta_queue_items[0].guta_stats,
                },
                top_line_siblings: [guta_queue_items[0].top_line_proof.siblings.clone(), old_mp.siblings].concat(),
            };
            tracing::debug!(rw = %serde_json::to_string_pretty(&rw).unwrap(), "Register witness");
            let r_with_deps = CircuitInputWithDependencies {
                input: rw,
                dependencies: vec![guta_queue_items[0].proof_id],
            };
            tracing::debug!(guta_item = %serde_json::to_string_pretty(&guta_queue_items[0]).unwrap(), "First GUTA queue item");

            let id = QProvingJobDataID::new(
                QJobTopic::GenerateStandardProof,
                checkpoint_id,
                slot_id,
                self.coordinator_config.coordinator_id,
                0,
                0,
                ProvingJobCircuitType::GUTAVerifyToCap,
                ProvingJobDataType::InputWitness,
                0,
            );

            let new_nodes = guta_queue_items
                .iter()
                .map(|x| {
                    assert_eq!(x.top_line_proof.index, x.realm_id, "right now guta proofs with top line are not allowed");
                    SimpleMerkleNode {
                        key: SimpleMerkleNodeKey {
                            level: self.coordinator_config.realm_root_level,
                            index: x.realm_id,
                        },
                        value: x.top_line_proof.new_root,
                    }
                })
                .collect::<Vec<_>>();

            let mut res = self.store.injest_user_tree_nodes_imm(checkpoint_id, 0, &new_nodes).await?;
            tracing::debug!(res = %serde_json::to_string_pretty(&res).unwrap(), "GUTA result");
            let good_old = self
                .store
                .get_user_top_tree_cap_root(checkpoint_id - 1, res.nearest_common_ancestor_level, res.nearest_common_ancestor_index)
                .await?;
            tracing::debug!(good_old = %good_old, "Good old value");

            res.link_proof.old_value = good_old;

            self.proof_store
                .set_bytes_by_id(id.get_input_witness_id(), &bincode::serialize(&r_with_deps)?)
                .await?;

            let mut graph = BidirectionalGraph::new();
            graph.add_node(id.get_output_id());
            for dep in &r_with_deps.dependencies {
                graph.add_edge(id.get_output_id(), *dep);
            }
            return Ok((vec![vec![id]], r_with_deps.input.get_new_guta_header::<PsyHasher>(), graph, offset_states));
        }

        // TODO: OPT: Maybe use a sorted queue/zset so we don't have to sort after we
        // drain
        guta_queue_items.sort_by(|a, b| a.realm_id.cmp(&b.realm_id));
        tracing::debug!(guta_queue_items = %serde_json::to_string_pretty(&guta_queue_items).unwrap(), "All GUTA queue items");

        let mut graph = BidirectionalGraph::new();

        let new_nodes = guta_queue_items
            .iter()
            .map(|x| {
                assert_eq!(x.top_line_proof.index, x.realm_id, "right now guta proofs with top line are not allowed");
                SimpleMerkleNode {
                    key: SimpleMerkleNodeKey {
                        level: self.coordinator_config.realm_root_level,
                        index: x.realm_id,
                    },
                    value: x.top_line_proof.new_root,
                }
            })
            .collect::<Vec<_>>();

        let res = self.store.injest_user_tree_nodes_imm(checkpoint_id, 0, &new_nodes).await?;
        tracing::debug!(res = %serde_json::to_string_pretty(&res).unwrap(), "GUTA aggregation result");

        let mut updates = Vec::with_capacity(res.nca_proofs.len());
        let mut combo_stats = Vec::with_capacity(res.nca_proofs.len());

        for (i, p) in res.nca_proofs.iter().enumerate() {
            tracing::debug!(i = i, verify_result = ?res.nca_proofs[i].verify::<PsyHasher>(), "NCA proof verification");
            let (l_dep_ind, r_dep_ind) = res.dependencies[i];
            if l_dep_ind <= -1 && r_dep_ind <= -1 {
                let l_dep_ind = -(l_dep_ind + 1) as usize;
                let r_dep_ind = -(r_dep_ind + 1) as usize;
                if guta_queue_items[l_dep_ind].checkpoint_tree_root != last_checkpoint_tree_root
                    || guta_queue_items[r_dep_ind].checkpoint_tree_root != last_checkpoint_tree_root
                {
                    tracing::debug!("LeftRightRealmGutaWithCheckpointUpgrade");
                    let real_left_guta_checkpoint_id = guta_queue_items[l_dep_ind].checkpoint_id.saturating_sub(1);
                    let real_right_guta_checkpoint_id = guta_queue_items[r_dep_ind].checkpoint_id.saturating_sub(1);
                    let historical_checkpoint_proof_a = self
                        .store
                        .get_checkpoint_tree_merkle_proof(last_checkpoint_id, real_left_guta_checkpoint_id)
                        .await?;
                    let historical_checkpoint_proof_b = self
                        .store
                        .get_checkpoint_tree_merkle_proof(last_checkpoint_id, real_right_guta_checkpoint_id)
                        .await?;
                    let input = VerifyTwoGUTAProofUpgradeCheckpointStandardInputSimple {
                        historical_checkpoint_proof_a,
                        historical_checkpoint_proof_b,
                        stats_a: guta_queue_items[l_dep_ind].guta_stats,
                        stats_b: guta_queue_items[r_dep_ind].guta_stats,
                        nca_proof: res.nca_proofs[i].to_partial(),
                    };
                    tracing::debug!(input = %serde_json::to_string_pretty(&input).unwrap(), "Two GUTA upgrade input");

                    let x = CircuitInputWithDependencies {
                        input,
                        dependencies: vec![guta_queue_items[l_dep_ind].proof_id, guta_queue_items[r_dep_ind].proof_id],
                    };
                    x.input.check_witness()?;
                    let w_id = QProvingJobDataID::new(
                        QJobTopic::GenerateStandardProof,
                        checkpoint_id,
                        slot_id,
                        self.coordinator_config.coordinator_id,
                        p.nearest_common_ancestor_level as u32,
                        p.nearest_common_ancestor_index as u32,
                        ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade,
                        ProvingJobDataType::InputWitness,
                        0,
                    );

                    combo_stats.push((w_id.get_output_id(), x.input.get_combined_stats()));

                    for dep in &x.dependencies {
                        graph.add_edge(w_id.get_output_id(), *dep);
                    }

                    updates.push(KVQPair {
                        key: w_id,
                        value: bincode::serialize(&x)?,
                    });
                } else {
                    tracing::debug!("LeftRightRealmGuta");
                    let input = VerifyTwoGUTAProofGadgetStandardInputSimple {
                        checkpoint_tree_root: guta_queue_items[l_dep_ind].checkpoint_tree_root,
                        b_checkpoint_tree_root: guta_queue_items[r_dep_ind].checkpoint_tree_root,
                        stats_a: guta_queue_items[l_dep_ind].guta_stats,
                        stats_b: guta_queue_items[r_dep_ind].guta_stats,
                        nca_proof: res.nca_proofs[i].to_partial(),
                    };
                    tracing::debug!(input = %serde_json::to_string_pretty(&input).unwrap(), "Two GUTA input");

                    let x = CircuitInputWithDependencies {
                        input,
                        dependencies: vec![guta_queue_items[l_dep_ind].proof_id, guta_queue_items[r_dep_ind].proof_id],
                    };
                    x.input.check_witness()?;
                    let w_id = QProvingJobDataID::new(
                        QJobTopic::GenerateStandardProof,
                        checkpoint_id,
                        slot_id,
                        self.coordinator_config.coordinator_id,
                        p.nearest_common_ancestor_level as u32,
                        p.nearest_common_ancestor_index as u32,
                        ProvingJobCircuitType::GUTATwoGUTA,
                        ProvingJobDataType::InputWitness,
                        0,
                    );

                    combo_stats.push((w_id.get_output_id(), x.input.get_combined_stats()));

                    for dep in &x.dependencies {
                        graph.add_edge(w_id.get_output_id(), *dep);
                    }

                    updates.push(KVQPair {
                        key: w_id,
                        value: bincode::serialize(&x)?,
                    });
                }
            } else if r_dep_ind > -1 && l_dep_ind > -1 {
                tracing::debug!("LeftRightCoordinatorGuta");
                let (l_proof_id, l_stats) = combo_stats[l_dep_ind as usize];
                let (r_proof_id, r_stats) = combo_stats[r_dep_ind as usize];
                let x = CircuitInputWithDependencies {
                    input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                        checkpoint_tree_root: last_checkpoint_tree_root,
                        b_checkpoint_tree_root: last_checkpoint_tree_root,
                        stats_a: l_stats,
                        stats_b: r_stats,
                        nca_proof: res.nca_proofs[i].to_partial(),
                    },
                    dependencies: vec![l_proof_id.get_output_id(), r_proof_id.get_output_id()],
                };
                let w_id = QProvingJobDataID::new(
                    QJobTopic::GenerateStandardProof,
                    checkpoint_id,
                    slot_id,
                    self.coordinator_config.coordinator_id,
                    p.nearest_common_ancestor_level as u32,
                    p.nearest_common_ancestor_index as u32,
                    ProvingJobCircuitType::GUTATwoGUTA,
                    ProvingJobDataType::InputWitness,
                    0,
                );
                combo_stats.push((w_id.get_output_id(), l_stats.combine_with(&r_stats)));

                for dep in &x.dependencies {
                    graph.add_edge(w_id.get_output_id(), *dep);
                }

                updates.push(KVQPair {
                    key: w_id,
                    value: bincode::serialize(&x)?,
                });
            } else if l_dep_ind > -1 {
                let (l_proof_id, l_stats) = combo_stats[l_dep_ind as usize];
                let r_dep_ind = -(r_dep_ind + 1) as usize;
                let right_guta_item = &guta_queue_items[r_dep_ind];

                if right_guta_item.checkpoint_tree_root != last_checkpoint_tree_root {
                    tracing::debug!("LeftCoordinatorGutaRightRealmGutaWithCheckpointUpgrade");
                    let real_last_guta_checkpoint_id = right_guta_item.checkpoint_id.saturating_sub(1);
                    let historical_checkpoint_proof_a = self
                        .store
                        .get_checkpoint_tree_merkle_proof(last_checkpoint_id, last_checkpoint_id)
                        .await?;
                    let historical_checkpoint_proof_b = self
                        .store
                        .get_checkpoint_tree_merkle_proof(last_checkpoint_id, real_last_guta_checkpoint_id)
                        .await?;
                    let x = CircuitInputWithDependencies {
                        input: VerifyTwoGUTAProofUpgradeCheckpointStandardInputSimple {
                            historical_checkpoint_proof_a,
                            historical_checkpoint_proof_b,
                            stats_a: l_stats,
                            stats_b: right_guta_item.guta_stats.clone(),
                            nca_proof: res.nca_proofs[i].to_partial(),
                        },
                        dependencies: vec![l_proof_id.get_output_id(), right_guta_item.proof_id],
                    };
                    x.input.check_witness()?;
                    let w_id = QProvingJobDataID::new(
                        QJobTopic::GenerateStandardProof,
                        checkpoint_id,
                        slot_id,
                        self.coordinator_config.coordinator_id,
                        p.nearest_common_ancestor_level as u32,
                        p.nearest_common_ancestor_index as u32,
                        ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade,
                        ProvingJobDataType::InputWitness,
                        0,
                    );

                    combo_stats.push((w_id.get_output_id(), x.input.get_combined_stats()));

                    for dep in &x.dependencies {
                        graph.add_edge(w_id.get_output_id(), *dep);
                    }

                    updates.push(KVQPair {
                        key: w_id,
                        value: bincode::serialize(&x)?,
                    });
                } else {
                    tracing::debug!("LeftCoordinatorGutaRightRealmGuta");
                    let x = CircuitInputWithDependencies {
                        input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                            checkpoint_tree_root: last_checkpoint_tree_root,
                            b_checkpoint_tree_root: last_checkpoint_tree_root,
                            stats_a: l_stats,
                            stats_b: right_guta_item.guta_stats.clone(),
                            nca_proof: res.nca_proofs[i].to_partial(),
                        },
                        dependencies: vec![l_proof_id.get_output_id(), right_guta_item.proof_id],
                    };
                    x.input.check_witness()?;
                    let w_id = QProvingJobDataID::new(
                        QJobTopic::GenerateStandardProof,
                        checkpoint_id,
                        slot_id,
                        self.coordinator_config.coordinator_id,
                        p.nearest_common_ancestor_level as u32,
                        p.nearest_common_ancestor_index as u32,
                        ProvingJobCircuitType::GUTATwoGUTA,
                        ProvingJobDataType::InputWitness,
                        0,
                    );

                    combo_stats.push((w_id.get_output_id(), x.input.get_combined_stats()));

                    for dep in &x.dependencies {
                        graph.add_edge(w_id.get_output_id(), *dep);
                    }

                    updates.push(KVQPair {
                        key: w_id,
                        value: bincode::serialize(&x)?,
                    });
                }
            } else {
                // -1 0
                //   2 3
                let (r_proof_id, r_stats) = combo_stats[r_dep_ind as usize];
                let l_dep_ind = -(l_dep_ind + 1) as usize;
                let left_guta_item = &guta_queue_items[l_dep_ind];

                if left_guta_item.checkpoint_tree_root != last_checkpoint_tree_root {
                    tracing::debug!("LeftRealmGutaRightCoordinatorGutaWithCheckpointUpgrade");
                    let real_last_guta_checkpoint_id = left_guta_item.checkpoint_id.saturating_sub(1);
                    let historical_checkpoint_proof_a = self
                        .store
                        .get_checkpoint_tree_merkle_proof(last_checkpoint_id, real_last_guta_checkpoint_id)
                        .await?;
                    let historical_checkpoint_proof_b = self
                        .store
                        .get_checkpoint_tree_merkle_proof(last_checkpoint_id, last_checkpoint_id)
                        .await?;
                    let x = CircuitInputWithDependencies {
                        input: VerifyTwoGUTAProofUpgradeCheckpointStandardInputSimple {
                            historical_checkpoint_proof_a,
                            historical_checkpoint_proof_b,
                            stats_a: left_guta_item.guta_stats.clone(),
                            stats_b: r_stats,
                            nca_proof: res.nca_proofs[i].to_partial(),
                        },
                        dependencies: vec![left_guta_item.proof_id, r_proof_id.get_output_id()],
                    };
                    x.input.check_witness()?;
                    let w_id = QProvingJobDataID::new(
                        QJobTopic::GenerateStandardProof,
                        checkpoint_id,
                        slot_id,
                        self.coordinator_config.coordinator_id,
                        p.nearest_common_ancestor_level as u32,
                        p.nearest_common_ancestor_index as u32,
                        ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade,
                        ProvingJobDataType::InputWitness,
                        0,
                    );

                    combo_stats.push((w_id.get_output_id(), x.input.get_combined_stats()));

                    for dep in &x.dependencies {
                        graph.add_edge(w_id.get_output_id(), *dep);
                    }

                    updates.push(KVQPair {
                        key: w_id,
                        value: bincode::serialize(&x)?,
                    });
                } else {
                    tracing::debug!("LeftRealmGutaRightCoordinatorGuta");
                    let x = CircuitInputWithDependencies {
                        input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                            checkpoint_tree_root: last_checkpoint_tree_root,
                            b_checkpoint_tree_root: last_checkpoint_tree_root,
                            stats_a: left_guta_item.guta_stats.clone(),
                            stats_b: r_stats,
                            nca_proof: res.nca_proofs[i].to_partial(),
                        },
                        dependencies: vec![left_guta_item.proof_id, r_proof_id.get_output_id()],
                    };
                    x.input.check_witness()?;
                    let w_id = QProvingJobDataID::new(
                        QJobTopic::GenerateStandardProof,
                        checkpoint_id,
                        slot_id,
                        self.coordinator_config.coordinator_id,
                        p.nearest_common_ancestor_level as u32,
                        p.nearest_common_ancestor_index as u32,
                        ProvingJobCircuitType::GUTATwoGUTA,
                        ProvingJobDataType::InputWitness,
                        0,
                    );

                    combo_stats.push((w_id.get_output_id(), x.input.get_combined_stats()));

                    for dep in &x.dependencies {
                        graph.add_edge(w_id.get_output_id(), *dep);
                    }

                    updates.push(KVQPair {
                        key: w_id,
                        value: bincode::serialize(&x)?,
                    });
                }
            }
        }

        self.proof_store.set_bytes_by_id_batch(&updates).await?;

        let mut levels = res
            .get_index_levels()
            .iter()
            .map(|l| l.iter().map(|x| combo_stats[*x].0).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        let mut guta = GlobalUserTreeAggregatorHeader {
            guta_circuit_whitelist: self.coordinator_config.guta_circuit_whitelist,
            checkpoint_tree_root: last_checkpoint_tree_root,
            state_transition: SubTreeNodeStateTransition {
                old_node_value: res.nca_proofs[res.root_proof_index].old_nearest_common_ancestor_value,
                new_node_value: res.nca_proofs[res.root_proof_index].new_nearest_common_ancestor_value,
                node_index: F::from_canonical_u64(res.nca_proofs[res.root_proof_index].nearest_common_ancestor_index),
                node_level: F::from_canonical_u8(res.nca_proofs[res.root_proof_index].nearest_common_ancestor_level),
            },
            stats: combo_stats[res.root_proof_index].1,
        };
        tracing::debug!(guta = %serde_json::to_string_pretty(&guta).unwrap(), guta_hash = %guta.qfhash::<PsyHasher>(), "Final GUTA");

        if guta.state_transition.node_level != F::ZERO {
            let w = CircuitInputWithDependencies::<VerifyGUTAToCapCircuitInputSimple<F>> {
                input: VerifyGUTAToCapCircuitInputSimple {
                    guta_proof_header: guta.clone(),
                    top_line_siblings: res.link_proof.siblings,
                },
                dependencies: vec![*(levels.last().as_ref().unwrap().last().unwrap())],
            };
            let w_id = QProvingJobDataID::new(
                QJobTopic::GenerateStandardProof,
                checkpoint_id,
                slot_id,
                self.coordinator_config.coordinator_id,
                0,
                0,
                ProvingJobCircuitType::GUTAVerifyToCap,
                ProvingJobDataType::InputWitness,
                0,
            );

            graph.add_edge(w_id.get_output_id(), w.dependencies[0].get_output_id());

            self.proof_store
                .set_bytes_by_id(w_id, &bincode::serialize(&w).map_err(|e| anyhow::anyhow!("{:?}", e))?)
                .await?;
            levels.push(vec![w_id]);

            guta = GlobalUserTreeAggregatorHeader {
                guta_circuit_whitelist: guta.guta_circuit_whitelist,
                checkpoint_tree_root: guta.checkpoint_tree_root,
                state_transition: SubTreeNodeStateTransition {
                    old_node_value: res.link_proof.old_root,
                    new_node_value: res.link_proof.new_root,
                    node_index: F::ZERO,
                    node_level: F::ZERO,
                },
                stats: guta.stats,
            };
            tracing::debug!(guta = %serde_json::to_string_pretty(&guta).unwrap(), guta_hash = %guta.qfhash::<PsyHasher>(), "GUTA subtree");
        }

        Ok((levels, guta, graph, offset_states))
    }

    pub async fn plan_jobs(
        &self,
        new_checkpoint_id: u64,
        slot_id: u64,
        user_registration_jobs: &Vec<Vec<QProvingJobDataID>>,
        deploy_jobs: &Vec<Vec<QProvingJobDataID>>,
        guta_jobs: &Vec<Vec<QProvingJobDataID>>,
    ) -> anyhow::Result<(QProvingJobDataID, QProvingJobDataID)> {
        tracing::debug!("Generated GUTA jobs: {:#?}", guta_jobs);
        tracing::debug!("Generated Deploy jobs: {:#?}", deploy_jobs);
        tracing::debug!("Generated registration jobs: {:#?}", user_registration_jobs);
        let notify_block_complete = QProvingJobDataID::notify_block_complete(new_checkpoint_id, slot_id, self.coordinator_config.coordinator_id);
        let root_state_transition =
            QProvingJobDataID::block_state_transition_input_witness(new_checkpoint_id, slot_id, self.coordinator_config.coordinator_id);
        let root_state_transition_task = QProvingTask::new(&[root_state_transition]);
        let notify_block_complete_task = QProvingTask::new(&[notify_block_complete]);
        self.task_store
            .write_next_tasks(&root_state_transition_task, &notify_block_complete_task)
            .await?;

        let state_part_1_common_id =
            QProvingJobDataID::get_block_aggregate_jobs_group(new_checkpoint_id, slot_id, self.coordinator_config.coordinator_id, 0);
        let state_part_1_id =
            QProvingJobDataID::block_agg_state_part_1_input_witness(new_checkpoint_id, slot_id, self.coordinator_config.coordinator_id);
        let state_part_1_task = QProvingTask::new(&[state_part_1_id]);
        let state_part_1_common_task = QProvingTask::new(&[state_part_1_common_id]);
        self.task_store
            .write_next_tasks(&state_part_1_common_task, &root_state_transition_task)
            .await?;
        self.task_store.write_next_tasks(&state_part_1_task, &state_part_1_common_task).await?;

        let register_users_agg_job_id =
            QProvingJobDataID::get_block_aggregate_jobs_group(new_checkpoint_id, slot_id, self.coordinator_config.coordinator_id, 1);
        let deploy_contracts_agg_job_id =
            QProvingJobDataID::get_block_aggregate_jobs_group(new_checkpoint_id, slot_id, self.coordinator_config.coordinator_id, 2);
        let guta_agg_job_id =
            QProvingJobDataID::get_block_aggregate_jobs_group(new_checkpoint_id, slot_id, self.coordinator_config.coordinator_id, 3);
        let register_users_agg_task = QProvingTask::new(&[register_users_agg_job_id]);
        let deploy_contracts_agg_task = QProvingTask::new(&[deploy_contracts_agg_job_id]);
        let guta_agg_task = QProvingTask::new(&[guta_agg_job_id]);

        self.task_store.write_next_tasks(&register_users_agg_task, &state_part_1_task).await?;
        self.task_store.write_next_tasks(&deploy_contracts_agg_task, &state_part_1_task).await?;
        self.task_store.write_next_tasks(&guta_agg_task, &state_part_1_task).await?;

        let user_registration_tasks = user_registration_jobs.iter().map(|jobs| QProvingTask::new(jobs)).collect::<Vec<_>>();
        let deploy_contracts_tasks = deploy_jobs.iter().map(|jobs| QProvingTask::new(jobs)).collect::<Vec<_>>();
        let guta_tasks = guta_jobs.iter().map(|jobs| QProvingTask::new(jobs)).collect::<Vec<_>>();
        self.task_store
            .write_multidimensional_tasks(&user_registration_tasks, &register_users_agg_task)
            .await?;
        self.task_store
            .write_multidimensional_tasks(&deploy_contracts_tasks, &deploy_contracts_agg_task)
            .await?;
        self.task_store.write_multidimensional_tasks(&guta_tasks, &guta_agg_task).await?;

        // Finalize and save the task topology
        debug!("plan_jobs for coordinator  , checkpoint_id {}", new_checkpoint_id);
        self.task_store.finalize_and_save_topology().await?;

        Ok((state_part_1_id, root_state_transition))
    }

    pub async fn build_block(&self, slot: u64) -> anyhow::Result<Vec<QueueOffsetState>> {
        let start = Instant::now();
        info!("coordinator STARTED new block");

        self.task_store.clear_task_graph().await?;

        let last_blockstate = self.store.get_latest_block_state().await?;
        self.proof_store
            .cleanup_old_proofs(last_blockstate.checkpoint_id, MAX_CHECKPOINT_COUNT as u64)
            .await?;
        let last_user_registration_tree_root = self.store.get_user_registration_tree_root(last_blockstate.checkpoint_id).await?;
        let last_contract_tree_root = self.store.get_contract_tree_root(last_blockstate.checkpoint_id).await?;
        let last_checkpoint_leaf = self.store.get_checkpoint_leaf_data(last_blockstate.checkpoint_id).await?;
        let new_checkpoint_id = last_blockstate.checkpoint_id + 1;
        info!("💥 coordinator processor build block checkpoint_id: {}", new_checkpoint_id);
        let (deploy_jobs, deploy_transition, next_contract_id, deploy_graph, deploy_contract_offset_state) =
            self.handle_deploy_contracts(new_checkpoint_id, slot).await?;
        let (
            user_registration_jobs,
            user_registration_transition,
            new_accounts,
            regsitered_users_start_pivot_siblings,
            user_reg_graph,
            register_user_offset_state,
        ) = self.handle_user_registrations(new_checkpoint_id, slot).await?;

        let (guta_jobs, guta_transition, guta_graph, mut offset_states) = self.handle_guta_from_realms(new_checkpoint_id, slot).await?;
        offset_states.push(deploy_contract_offset_state);
        offset_states.push(register_user_offset_state);
        // Set the job dependency graphs
        self.task_store.set_job_dependency_graph(deploy_graph, user_reg_graph, guta_graph).await?;

        let root_deploy_job = deploy_jobs
            .last()
            .and_then(|jobs| jobs.last())
            .ok_or_else(|| anyhow::anyhow!("No deploy contract jobs found"))?;
        let root_user_registration_job = user_registration_jobs
            .last()
            .and_then(|jobs| jobs.last())
            .ok_or_else(|| anyhow::anyhow!("No user registration jobs found"))?;
        let root_guta_job = guta_jobs
            .last()
            .and_then(|jobs| jobs.last())
            .ok_or_else(|| anyhow::anyhow!("No GUTA jobs found"))?;

        tracing::info!(new_checkpoint_id = new_checkpoint_id, "Building new checkpoint");
        let (state_part_1_id, root_state_transition) = self
            .plan_jobs(new_checkpoint_id, slot, &user_registration_jobs, &deploy_jobs, &guta_jobs)
            .await?;

        debug!(
            "Waiting for user registration aggregation job to complete for checkpoint {}",
            new_checkpoint_id
        );
        debug!(
            "Waiting for deploy contracts aggregation job to complete for checkpoint {}",
            new_checkpoint_id
        );
        debug!("Waiting for GUTA aggregation job to complete for checkpoint {}", new_checkpoint_id);
        // let remaining_time =
        // Some(Duration::from_millis(LocalClock.get_current_slot_remaining_time()));
        let remaining_time = Some(Duration::from_millis(5 * SLOT_SIZE));
        let (register_users_proof, deploy_contracts_proof, guta_proof) = match tokio::join!(
            self.prover_queue.wait_for_job_proof::<C, D>(*root_user_registration_job, remaining_time),
            self.prover_queue.wait_for_job_proof::<C, D>(*root_deploy_job, remaining_time),
            self.prover_queue.wait_for_job_proof::<C, D>(*root_guta_job, remaining_time),
        ) {
            (Ok(register_users_proof), Ok(deploy_contracts_proof), Ok(guta_proof)) => (register_users_proof, deploy_contracts_proof, guta_proof),
            (Err(e), _, _) => anyhow::bail!("Failed to wait for register users job proofs: {}", e),
            (_, Err(e), _) => anyhow::bail!("Failed to wait for deploy contracts job proofs: {}", e),
            (_, _, Err(e)) => anyhow::bail!("Failed to wait for GUTA job proofs: {}", e),
        };

        let part_1_input = CircuitInputWithDependencies {
            input: QCAggUserRegistartionDeployContractsGUTAInput {
                register_users_state_transition: if user_registration_transition.state_transition_start == QHashOut::ZERO {
                    AggStateTransition {
                        state_transition_start: last_user_registration_tree_root,
                        state_transition_end: last_user_registration_tree_root,
                    }
                } else {
                    user_registration_transition
                },
                deploy_contracts_state_transition: if deploy_transition.state_transition_start == QHashOut::ZERO {
                    AggStateTransition {
                        state_transition_start: last_contract_tree_root,
                        state_transition_end: last_contract_tree_root,
                    }
                } else {
                    deploy_transition
                },
                guta_proof_header: guta_transition,
            },
            dependencies: vec![
                root_user_registration_job.get_output_id(),
                root_deploy_job.get_output_id(),
                root_guta_job.get_output_id(),
            ],
        };

        tracing::debug!(part_1_input = %serde_json::to_string_pretty(&part_1_input).unwrap(), "Part 1 input for AggUserRegisterDeployContractsGUTA");
        self.proof_store
            .set_bytes_by_id(
                state_part_1_id.get_input_witness_id(),
                &bincode::serialize(&part_1_input).map_err(|e| anyhow::anyhow!("{:?}", e))?,
            )
            .await?;

        let register_users_commitment = QHashOut::try_from(&register_users_proof.public_inputs[0..4])?;
        let register_users_worker_public_key = QHashOut::try_from(&register_users_proof.public_inputs[4..8])?;
        let deploy_contracts_commitment = QHashOut::try_from(&deploy_contracts_proof.public_inputs[0..4])?;
        let deploy_contracts_worker_public_key = QHashOut::try_from(&deploy_contracts_proof.public_inputs[4..8])?;
        let gutas_commitment = QHashOut::try_from(&guta_proof.public_inputs[0..4])?;
        let gutas_worker_public_key = QHashOut::try_from(&guta_proof.public_inputs[4..8])?;

        let pm_rewards_commitment = PMRewardCommitment {
            register_users_root: QHashOut(PsyHasher::two_to_one(
                register_users_commitment.into(),
                register_users_worker_public_key.into(),
            )),
            deploy_contracts_root: QHashOut(PsyHasher::two_to_one(
                deploy_contracts_commitment.into(),
                deploy_contracts_worker_public_key.into(),
            )),
            gutas_root: QHashOut(PsyHasher::two_to_one(gutas_commitment.into(), gutas_worker_public_key.into())),
        };

        let register_users_stats = PMJobsCompletedStats {
            deploy_contracts_completed: register_users_proof.public_inputs[8],
            register_users_completed: register_users_proof.public_inputs[9],
            gutas_completed: register_users_proof.public_inputs[10],
        };
        let deploy_contracts_stats = PMJobsCompletedStats {
            deploy_contracts_completed: deploy_contracts_proof.public_inputs[8],
            register_users_completed: deploy_contracts_proof.public_inputs[9],
            gutas_completed: deploy_contracts_proof.public_inputs[10],
        };
        let guta_stats = PMJobsCompletedStats {
            deploy_contracts_completed: guta_proof.public_inputs[8],
            register_users_completed: guta_proof.public_inputs[9],
            gutas_completed: guta_proof.public_inputs[10],
        };

        let pm_jobs_completed_stats = PMJobsCompletedStats {
            deploy_contracts_completed: register_users_stats.deploy_contracts_completed
                + deploy_contracts_stats.deploy_contracts_completed
                + guta_stats.deploy_contracts_completed,
            register_users_completed: register_users_stats.register_users_completed
                + deploy_contracts_stats.register_users_completed
                + guta_stats.register_users_completed,
            gutas_completed: register_users_stats.gutas_completed + deploy_contracts_stats.gutas_completed + guta_stats.gutas_completed,
        };

        let partial_input = QCPsyCheckpointStateTransitionInputPartial {
            part_1_header: part_1_input.input,
            old_stats: last_checkpoint_leaf.stats,
            block_time: F::from_canonical_u64(Utc::now().timestamp_millis() as u64),
            final_random_seed_contribution: QHashOut::rand(),
            pm_rewards_commitment,
            pm_jobs_completed: pm_jobs_completed_stats,
        };

        tracing::debug!(partial_input = %serde_json::to_string_pretty(&partial_input).unwrap(), "Checkpoint state transition partial input");
        let new_checkpoint_leaf = partial_input.get_new_checkpoint_leaf::<PsyHasher>();
        tracing::debug!(new_checkpoint_leaf = %serde_json::to_string_pretty(&new_checkpoint_leaf).unwrap(), "New checkpoint leaf");
        let new_checkpoint_leaf_hash = new_checkpoint_leaf.qfhash::<PsyHasher>();
        tracing::debug!(new_checkpoint_leaf_hash = %new_checkpoint_leaf_hash, "New checkpoint leaf hash");

        let previous_checkpoint_proof = self
            .store
            .get_checkpoint_tree_merkle_proof(last_blockstate.checkpoint_id, last_blockstate.checkpoint_id)
            .await?;
        tracing::debug!(previous_checkpoint_proof = %serde_json::to_string_pretty(&previous_checkpoint_proof).unwrap(), "Previous checkpoint proof");

        let checkpoint_dmp = self
            .store
            .set_checkpoint_tree_leaf_hash_imm(new_checkpoint_id, new_checkpoint_leaf_hash)
            .await?;
        tracing::debug!(checkpoint_dmp = %serde_json::to_string_pretty(&checkpoint_dmp).unwrap(), "Checkpoint DMP");

        let witness_checkpoint_state_transition = CircuitInputWithDependencies {
            input: QCPsyCheckpointStateTransitionInput::<F> {
                partial: partial_input,
                append_checkpoint_tree_proof: checkpoint_dmp.clone(),
                previous_checkpoint_proof,
            },
            dependencies: vec![state_part_1_id.get_output_id()],
        };

        tracing::debug!(witness_checkpoint_state_transition = %serde_json::to_string_pretty(&witness_checkpoint_state_transition).unwrap(), "Checkpoint state transition witness");
        self.proof_store
            .set_bytes_by_id(
                root_state_transition.get_input_witness_id(),
                &bincode::serialize(&witness_checkpoint_state_transition).map_err(|e| anyhow::anyhow!("{:?}", e))?,
            )
            .await?;

        // Wait for final block proving jobs
        debug!("Waiting for block proving jobs for checkpoint {}", new_checkpoint_id);
        self.prover_queue
            .wait_for_block_proving_jobs_imm(new_checkpoint_id, Some(Duration::from_millis(5 * SLOT_SIZE)))
            .await?;
        debug!("Block proving jobs completed for checkpoint {}", new_checkpoint_id);

        // Update block state
        let new_block_state = PsyBlockState {
            checkpoint_id: last_blockstate.checkpoint_id + 1,
            next_add_withdrawal_id: last_blockstate.next_add_withdrawal_id,
            next_process_withdrawal_id: last_blockstate.next_process_withdrawal_id,
            next_deposit_id: last_blockstate.next_deposit_id,
            total_deposits_claimed_epoch: last_blockstate.total_deposits_claimed_epoch,
            next_user_id: last_blockstate.next_user_id + new_accounts.len() as u64,
            end_balance: last_blockstate.end_balance,
            next_contract_id,
        };

        // Save checkpoint data
        self.store.set_checkpoint_leaf_data_imm(new_checkpoint_id, &new_checkpoint_leaf).await?;
        self.store.set_block_state_imm(&new_block_state).await?;

        let lf_state = self.store.get_checkpoint_global_state_roots(new_checkpoint_id).await?;

        let sync_info = QCheckpointSyncInfoCompact {
            block_state: new_block_state,
            stats: new_checkpoint_leaf.stats,
            state_roots: lf_state,
            checkpoint_tree_update_siblings: checkpoint_dmp.siblings.clone(),
            regsitered_users_start_pivot_siblings,
            registered_users: new_accounts,
            old_checkpoint_leaf_hash: checkpoint_dmp.old_value,
            slot,
        };

        tracing::debug!(sync_info = %serde_json::to_string_pretty(&sync_info).unwrap(), "Checkpoint sync info");
        self.store.set_checkpoint_sync_info_imm(sync_info.clone()).await?;
        trace!("build block {}, slot {}, cost time: {:?}", new_checkpoint_id, slot, start.elapsed());
        Ok(offset_states)
    }

    pub async fn has_pending_tasks(&self, checkpoint_id: u64) -> anyhow::Result<bool> {
        let deploy_count = self
            .checkpoint_queue
            .cdq_len_imm(self.coordinator_config.deploy_contract_channel_id)
            .await?;

        trace!("Checking deploy contracts queue: {} items, checkpoint: {}", deploy_count, checkpoint_id);
        if deploy_count > 0 {
            return Ok(true);
        }

        let user_reg_count = self.checkpoint_queue.cdq_len_imm(COORD_API_REGISTER_USER_CHANNEL_ID).await?;

        trace!(
            "Checking user registration queue: {} items, checkpoint: {}",
            user_reg_count,
            checkpoint_id
        );
        if user_reg_count > 0 {
            return Ok(true);
        }

        let guta_count = self.checkpoint_queue.cdq_len_imm(self.coordinator_config.guta_channel_id).await?;

        trace!("Checking GUTA queue: {} items, checkpoint: {}", guta_count, checkpoint_id);
        if guta_count > 0 {
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn commit(
        &self,
        checkpoint_id: u64,
        offset_states: Vec<QueueOffsetState>,
    ) -> anyhow::Result<(Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        let (pair_to_set, remove_keys) = self.store.commit(None)?;
        for offset_state in offset_states.iter() {
            self.checkpoint_queue.commit_offset(offset_state).await?;
        }
        self.task_store
            .save_job_dependency_graph(checkpoint_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to save job dependency graph for checkpoint {}: {}", checkpoint_id, e))?;
        Ok((pair_to_set, remove_keys))
    }

    pub async fn rollback(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        self.task_store.clear_job_dependency_graph(checkpoint_id).await?;
        self.store.rollback(checkpoint_id)
    }
}
