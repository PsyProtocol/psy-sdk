use std::{sync::Arc, time::Instant};

use chrono::Utc;
use kvq::traits::KVQPair;
use plonky2::{
    field::types::Field,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use qed_core::{
    config::network_constants::{
        BATCH_DEPLOY_CONTRACT_SUB_TREE_HEIGHT, BATCH_USER_REGISTRAITION_MAX_SUB_TREES, BATCH_USER_REGISTRAITION_SUB_TREE_HEIGHT, COORDINATOR_USER_TREE_HEIGHT, COORD_API_DEPLOY_CONTRACT_CHANNEL_ID, COORD_API_GUTA_FROM_REALMS_CHANNEL_ID, COORD_API_REGISTER_USER_CHANNEL_ID, DA_CHALLENGE_WINDOW, REALM_USER_TREE_HEIGHT
    },
    data::qhashout::QHashOut,
    job::{
        drain_queue::{CheckpointDrainQueueConsumerAsyncImm, WithDrainQueueMetadata},
        history_queue::CheckpointHistoryQueueEmitterAsyncImm,
        id::{QProvingTask, ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID},
        traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
        worker_queue::WorkerEventTransmitterAsyncImm,
    },
};
use qed_crypto::{
    common::{
        cached_circuit_library::get_cached_circuit_library,
        circuit_library::CircuitInfoLibraryCore, generic_circuit_verifier::GenericCircuitVerifier, user_id::get_user_id_from_registration_id,
    },
    hash::{
        merkle::{
            treeprover::{
                data::CircuitInputWithDependencies, subtree::SubTreeNodeStateTransition,
                tree_helper::plan_tree_prover_from_leaves, AggStateTransition,
                AggStateTransitionInput, AggWTLeafAggregator,
            },
            utils::common::{SimpleMerkleNode, SimpleMerkleNodeKey},
        },
        traits::{
            hasher::FieldQHasher,
            qhashable::QFieldHashable,
        },
    },
    signature::zk::data::ZKPublicKeyInfo,
};
use qed_data::{
    config::store_config::UserPublicKeyTableStore, guta::{
        api::SubmitGUTARealmResultAPIQueueItem,
        header::GlobalUserTreeAggregatorHeader,
        proof_input::{
            GUTANoChangeFullInput, VerifyGUTAToCapCircuitInputSimple,
            VerifyTwoGUTAProofGadgetStandardInputSimple,
        },
        stats::GUTAStats,
    }, models::checkpoint::user_public_keys::QEDUserPublicKeyHelperModelCore, proof_store::builder::ProofStoreBuilder, protocol::circuit_inputs::{agg_part_1::QCAggUserRegistartionDeployContractsGUTAInput, checkpoint_transition::{QCQEDCheckpointStateTransitionInput, QCQEDCheckpointStateTransitionInputPartial}}, qblock::cmds::deploy_contract::QBCDeployContractWithRoot, qdata::{
        checkpoint::{
            QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf,
            QEDCheckpointLeafCompactWithStateRoots, QEDCheckpointLeafStats, QEDL2BlockState,
        },
        contract::QEDContractLeaf,
        pm_reward_commitment::PMRewardCommitment, user_public_key::QEDUserPublicKeyRecord,
    }
};
use qed_rollup_circuit::guta::gadgets::guta_header;
use qed_data::{
    config::store_config::{QCheckpointSyncInfoCompact, QEDFelt, QEDHasher, UserTreeStore}, models::kvq_merkle::model::KVQFixedConfigMerkleTreeModelReaderCore,
};
use qed_store::{
    node::coordinator::{
        QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm,
    },
    queue::task_queue::QProvingTaskStore,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use std::marker::Sync;


type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CoordinatorConfig {
    pub rpc_node_id: u32,
    pub users_per_realm: usize,
    pub realm_root_level: u8,

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
    pub fn get_standard(rpc_node_id: u32) -> Self {
        let library = get_cached_circuit_library::<F>();

        let realm_root_level = COORDINATOR_USER_TREE_HEIGHT;
        let users_per_realm = 1usize << (REALM_USER_TREE_HEIGHT as usize);

        Self {
            rpc_node_id,
            users_per_realm,
            realm_root_level,
            guta_channel_id: COORD_API_GUTA_FROM_REALMS_CHANNEL_ID,
            register_user_channel_id: COORD_API_REGISTER_USER_CHANNEL_ID,
            deploy_contract_channel_id: COORD_API_DEPLOY_CONTRACT_CHANNEL_ID,
            register_user_tree_batch_height: BATCH_USER_REGISTRAITION_SUB_TREE_HEIGHT as u8,
            register_user_tree_batch_size: BATCH_USER_REGISTRAITION_MAX_SUB_TREES,
            deploy_contracts_tree_batch_height: BATCH_DEPLOY_CONTRACT_SUB_TREE_HEIGHT as u8,
            register_users_circuit_whitelist: library
                .get_agg_whitelist::<QEDHasher>(ProvingJobCircuitType::AppendUserRegistrationTree)
                .unwrap(),
            register_user_dummy_state_root: QHashOut::ZERO,
            deploy_contracts_circuit_whitelist: library
                .get_agg_whitelist::<QEDHasher>(ProvingJobCircuitType::BatchDeployContracts)
                .unwrap(),
            deploy_contracts_dummy_state_root: QHashOut::ZERO,
            guta_circuit_whitelist: library
                .get_group_inclusion_proof(
                    ProvingJobCircuitType::GUTATwoGUTA,
                    ProvingJobCircuitType::GUTATwoGUTA,
                )
                .unwrap()
                .root,
        }
    }
}
#[derive(Clone)]
pub struct CoordinatorProcessorContext<
    SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
    DQ: CheckpointDrainQueueConsumerAsyncImm,
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
}

impl<
        SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueConsumerAsyncImm,
        HQ: CheckpointHistoryQueueEmitterAsyncImm,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm + Sync,
        TS: QProvingTaskStore + Sync
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
        })
    }

    pub fn verify_proof_of_type(
        &self,
        circuit_type: ProvingJobCircuitType,
        proof: &ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<()> {
        self.proof_verifier
            .verify_proof_of_type(circuit_type, proof)
    }

    pub async fn handle_deploy_contracts(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<(
        Vec<Vec<QProvingJobDataID>>,
        AggStateTransition<F>,
        u32,
    )> {
        let last_l2_blockstate = self.store.get_latest_l2_block_state().await?;

        let last_contract_tree_root = self.store.get_contract_tree_root(checkpoint_id).await?;
        let deploy_contract_items = self
            .checkpoint_queue
            .cdq_drain_imm::<WithDrainQueueMetadata<QBCDeployContractWithRoot<F>>>(
                self.coordinator_config.deploy_contract_channel_id,
                checkpoint_id,
            )
            .await?;

        let new_contract_leaves = deploy_contract_items
            .iter()
            .map(|x| QEDContractLeaf {
                deployer: x.payload.deployer,
                function_tree_root: x.payload.function_whitelist_root,
                state_tree_height: F::from_canonical_u16(
                    x.payload.code_definition.state_tree_height,
                ),
            })
            .collect::<Vec<_>>();

        let new_hashes = new_contract_leaves
            .iter()
            .map(|x| x.qfhash::<QEDHasher>())
            .collect::<Vec<_>>();

        let start_contract_id = last_l2_blockstate.next_contract_id;

        for (i, dc) in deploy_contract_items.iter().enumerate() {
            self.store.set_contract_code_definition_imm(
                checkpoint_id,
                start_contract_id as u64+ i as u64,
                &dc.payload.code_definition,
            ).await?;
            self.store.set_contract_function_whitelist_imm(checkpoint_id, start_contract_id as u64+ i as u64, &dc.payload.function_whitelist).await?;

        }
        for (i, l) in new_contract_leaves.iter().enumerate() {
            self.store.set_contract_leaf_data_imm(checkpoint_id, start_contract_id as u64+ i as u64, l).await?;
        }
        let next_contract_id = start_contract_id + new_contract_leaves.len() as u32;
        let mut psb = ProofStoreBuilder::new();
        let wits = self
            .store
            .batch_append_contract_tree_imm(
                checkpoint_id,
                start_contract_id as u64,
                self.coordinator_config.deploy_contracts_tree_batch_height,
                &new_hashes,
            )
            .await?
            .into_iter()
            .zip(new_contract_leaves.chunks(
                1usize << (self.coordinator_config.deploy_contracts_tree_batch_height as usize),
            ))
            .enumerate()
            .map(|(i, (c, l))| {
                self.push_deploy_contracts_request(i as u32, checkpoint_id, &mut psb, c, l.to_vec())
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let (batch_deploy_contract_tree_job_ids, root_transition_batch_deploy_contract_tree) =
            plan_tree_prover_from_leaves::<
                F,
                ProofStoreBuilder,
                AggWTLeafAggregator,
                _,
                AggStateTransitionInput<F>,
            >(
                &wits,
                &mut psb,
                QProvingJobDataID::new_proof_job_id(
                    checkpoint_id,
                    ProvingJobCircuitType::DummyBatchDeployContractsAggregate,
                    0xDD,
                    0,
                    0,
                ),
                last_contract_tree_root,
                self.coordinator_config.deploy_contracts_circuit_whitelist,
            )?;

        self.proof_store.set_bytes_by_id_batch(&psb.kvs).await?;
        let new_contract_tree_root = self.store.get_contract_tree_root(checkpoint_id).await?;

        Ok((
            batch_deploy_contract_tree_job_ids,
            AggStateTransition {
                state_transition_start: last_contract_tree_root,
                state_transition_end: new_contract_tree_root,
            },
            next_contract_id,
        ))
    }

    pub async fn handle_user_registrations(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<(
        Vec<Vec<QProvingJobDataID>>,
        AggStateTransition<F>,
        Vec<ZKPublicKeyInfo<F>>,
        Vec<QHashOut<F>>,
    )> {
        let last_l2_blockstate = self.store.get_latest_l2_block_state().await?;

        let last_user_registration_tree_root = self.store.get_user_registration_tree_root(checkpoint_id).await?;
        let user_registrations = self
            .checkpoint_queue
            .cdq_drain_imm::<ZKPublicKeyInfo<F>>(COORD_API_REGISTER_USER_CHANNEL_ID, 0)
            .await?;

        let start_registration_user_id = last_l2_blockstate.next_user_id;

        let new_user_records = user_registrations.iter().enumerate().map(|(i, x)| {
            let registration_id = start_registration_user_id + (i as u64);
            let user_id = get_user_id_from_registration_id(registration_id);
            QEDUserPublicKeyRecord {
                public_key_param: x.public_key_param,
                fingerprint: x.fingerprint,
                public_key: x.qfhash::<QEDHasher>(),
                user_id,
                checkpoint_id,
            }
        }).collect::<Vec<_>>();
        tracing::info!(
            "injest_checkpoint_sync_data_imm: start_registration_user_id: {}, new_user_records: {:?}",
            start_registration_user_id,
            new_user_records
        );
        self.store.set_user_public_key_records(&new_user_records).await?;

        let new_public_keys = user_registrations
            .iter()
            .map(|x| x.to_hash::<QEDHasher>())
            .collect::<Vec<_>>();

        let mut psb = ProofStoreBuilder::new();
        let wits = self
            .store
            .batch_append_user_registration_tree_imm(
                checkpoint_id,
                start_registration_user_id,
                self.coordinator_config.register_user_tree_batch_height,
                &new_public_keys,
            )
            .await?
            .chunks(self.coordinator_config.register_user_tree_batch_size)
            .enumerate()
            .map(|(i, c)| {
                self.push_user_registration_request(i as u32, checkpoint_id, &mut psb, c.to_vec())
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

        let (append_user_registration_tree_job_ids, root_transition_append_user_registration_tree) =
            plan_tree_prover_from_leaves::<
                F,
                ProofStoreBuilder,
                AggWTLeafAggregator,
                _,
                AggStateTransitionInput<F>,
            >(
                &wits,
                &mut psb,
                QProvingJobDataID::new_proof_job_id(
                    checkpoint_id,
                    ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate,
                    0xDD,
                    0,
                    0,
                ),
                last_user_registration_tree_root,
                self.coordinator_config.register_users_circuit_whitelist,
            )?;

        self.proof_store.set_bytes_by_id_batch(&psb.kvs).await?;
        let new_user_registration_tree_root = self.store.get_user_registration_tree_root(checkpoint_id).await?;

        Ok((
            append_user_registration_tree_job_ids,
            AggStateTransition {
                state_transition_start: last_user_registration_tree_root,
                state_transition_end: new_user_registration_tree_root,
            },
            user_registrations,
            siblings,
        ))
    }

    pub async fn handle_guta_from_realms(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<(
        Vec<Vec<QProvingJobDataID>>,
        GlobalUserTreeAggregatorHeader<F>,
    )> {
        tracing::debug!(checkpoint_id = checkpoint_id, "Processing checkpoint");
        let mut guta_queue_items = self
            .checkpoint_queue
            .cdq_drain_imm::<SubmitGUTARealmResultAPIQueueItem<F>>(
                self.coordinator_config.guta_channel_id,
                checkpoint_id,
            )
            .await?;
        tracing::debug!(guta_queue_items = ?guta_queue_items, "GUTA queue items");

        if guta_queue_items.len() == 0 {
            tracing::debug!("No GUTA queue items");
            let last_checkpoint_id = if checkpoint_id == 0 {
                checkpoint_id
            } else {
                checkpoint_id - 1
            };
            let roots = self
                .store
                .get_checkpoint_global_state_roots(last_checkpoint_id)
                .await?;
            let checkpoint_tree_proof = self
                .store
                .get_checkpoint_tree_merkle_proof(checkpoint_id, last_checkpoint_id)
                .await?;
            let checkpoint_leaf = self
                .store
                .get_checkpoint_leaf_data(last_checkpoint_id)
                .await?;

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
            tracing::debug!(guta_header = ?guta_header, guta_header_hash = ?guta_header.qfhash::<QEDHasher>(), "GUTA header");
            let input = GUTANoChangeFullInput {
                checkpoint_tree_proof,
                checkpoint_leaf: QEDCheckpointLeafCompactWithStateRoots {
                    checkpoint_leaf: checkpoint_leaf.to_compact::<QEDHasher>(),
                    global_state_roots: roots,
                },
            };
            tracing::debug!(input = ?input, "Single GUTA input");

            let id = QProvingJobDataID::core_op_witness(
                ProvingJobCircuitType::GUTANoChange,
                checkpoint_id,
                0,
            );

            self.proof_store
                .set_bytes_by_id(id.get_input_witness_id(), &bincode::serialize(&input)?)
                .await?;

            return Ok((vec![vec![id]], guta_header));
        } else if guta_queue_items.len() == 1 {
            tracing::debug!("Processing single GUTA queue item");
            let old_mp = self
                .store
                .get_user_top_tree_merkle_proof(
                    checkpoint_id,
                    self.coordinator_config.realm_root_level,
                    guta_queue_items[0].realm_id,
                )
                .await?;

            let rw = VerifyGUTAToCapCircuitInputSimple {
                guta_proof_header: GlobalUserTreeAggregatorHeader {
                    guta_circuit_whitelist: self.coordinator_config.guta_circuit_whitelist,
                    checkpoint_tree_root: guta_queue_items[0].checkpoint_tree_root,
                    state_transition: SubTreeNodeStateTransition {
                        old_node_value: guta_queue_items[0].top_line_proof.old_root,
                        new_node_value: guta_queue_items[0].top_line_proof.new_root,
                        node_index: F::from_noncanonical_u64(
                            guta_queue_items[0].top_line_proof.index,
                        ),
                        node_level: F::from_canonical_u64(
                            (self.coordinator_config.realm_root_level as usize
                                + guta_queue_items[0].top_line_proof.siblings.len())
                                as u64,
                        ),
                    },
                    stats: guta_queue_items[0].guta_stats,
                },
                top_line_siblings: [
                    guta_queue_items[0].top_line_proof.siblings.clone(),
                    old_mp.siblings,
                ]
                .concat(),
            };
            tracing::debug!(rw = ?rw, "Register witness");
            let r_with_deps = CircuitInputWithDependencies {
                input: rw,
                dependencies: vec![guta_queue_items[0].proof_id],
            };
            tracing::debug!(guta_item = ?guta_queue_items[0], "First GUTA queue item");

            let id = QProvingJobDataID::core_op_witness(
                ProvingJobCircuitType::GUTAVerifyToCap,
                checkpoint_id,
                0,
            );

            let new_nodes = guta_queue_items
                .iter()
                .map(|x| {
                    assert_eq!(
                        x.top_line_proof.index, x.realm_id,
                        "right now guta proofs with top line are not allowed"
                    );
                    SimpleMerkleNode {
                        key: SimpleMerkleNodeKey {
                            level: self.coordinator_config.realm_root_level,
                            index: x.realm_id,
                        },
                        value: x.top_line_proof.new_root,
                    }
                })
                .collect::<Vec<_>>();

            let mut res = self
                .store
                .injest_user_tree_nodes_imm(checkpoint_id, 0, &new_nodes)
                .await?;
            tracing::debug!(res = ?res, "GUTA result");
            //for local testing conv
            let good_old = self.store.get_user_top_tree_cap_root(checkpoint_id-1, res.nearest_common_ancestor_level, res.nearest_common_ancestor_index).await?;
            tracing::debug!(good_old = ?good_old, "Good old value");

            res.link_proof.old_value = good_old;

            self.proof_store
                .set_bytes_by_id(
                    id.get_input_witness_id(),
                    &bincode::serialize(&r_with_deps)?,
                )
                .await?;

            return Ok((vec![vec![id]],r_with_deps.input.get_new_guta_header::<QEDHasher>()))// r_with_deps.input.guta_proof_header));
        }

        // TODO: OPT: Maybe use a sorted queue/zset so we don't have to sort after we drain
        guta_queue_items.sort_by(|a, b| a.realm_id.cmp(&b.realm_id));
        tracing::debug!(guta_queue_items = ?guta_queue_items, "All GUTA queue items");

        let new_nodes = guta_queue_items
            .iter()
            .map(|x| {
                assert_eq!(
                    x.top_line_proof.index, x.realm_id,
                    "right now guta proofs with top line are not allowed"
                );
                SimpleMerkleNode {
                    key: SimpleMerkleNodeKey {
                        level: self.coordinator_config.realm_root_level,
                        index: x.realm_id,
                    },
                    value: x.top_line_proof.new_root,
                }
            })
            .collect::<Vec<_>>();

        let res = self
            .store
            .injest_user_tree_nodes_imm(checkpoint_id, 0, &new_nodes)
            .await?;
        tracing::debug!(res = ?res, "GUTA aggregation result");

        let mut witnesses = Vec::with_capacity(res.nca_proofs.len());

        for (i, p) in res.nca_proofs.iter().enumerate() {
            tracing::debug!(i = i, verify_result = ?res.nca_proofs[i].verify::<QEDHasher>(), "NCA proof verification");
            let (l_dep_ind, r_dep_ind) = res.dependencies[i];
            if l_dep_ind == -1 && r_dep_ind == -1 {
                tracing::debug!("Both dependencies are new");
                let input = VerifyTwoGUTAProofGadgetStandardInputSimple {
                    checkpoint_tree_root: guta_queue_items[i * 2].checkpoint_tree_root,
                    b_checkpoint_tree_root: guta_queue_items[i * 2 + 1].checkpoint_tree_root,
                    stats_a: guta_queue_items[i * 2].guta_stats,
                    stats_b: guta_queue_items[i * 2 + 1].guta_stats,
                    nca_proof: res.nca_proofs[i].to_partial(),
                };
                tracing::debug!(input = ?input, "Two GUTA input");

                let x = CircuitInputWithDependencies {
                    input,
                    dependencies: vec![
                        guta_queue_items[i * 2].proof_id,
                        guta_queue_items[i * 2 + 1].proof_id,
                    ],
                };
                let w_id = QProvingJobDataID::new(
                    QJobTopic::GenerateStandardProof,
                    checkpoint_id,
                    ProvingJobCircuitType::GUTATwoGUTA.to_circuit_group_id(),
                    p.nearest_common_ancestor_level as u32,
                    p.nearest_common_ancestor_index as u32,
                    ProvingJobCircuitType::GUTATwoGUTA,
                    ProvingJobDataType::InputWitness,
                    0,
                );

                witnesses.push((w_id, x));
            } else if r_dep_ind != -1 && l_dep_ind != -1 {
                tracing::debug!("Both dependencies exist");
                let x = CircuitInputWithDependencies {
                    input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                        checkpoint_tree_root: witnesses[l_dep_ind as usize]
                            .1
                            .input
                            .checkpoint_tree_root,
                        b_checkpoint_tree_root: witnesses[r_dep_ind as usize].1.input.checkpoint_tree_root,
                        stats_a: witnesses[l_dep_ind as usize].1.input.get_combined_stats(),
                        stats_b: witnesses[r_dep_ind as usize].1.input.get_combined_stats(),
                        nca_proof: res.nca_proofs[i].to_partial(),
                    },
                    dependencies: vec![
                        witnesses[l_dep_ind as usize].0.get_output_id(),
                        witnesses[r_dep_ind as usize].0.get_output_id(),
                    ],
                };
                let w_id = QProvingJobDataID::new(
                    QJobTopic::GenerateStandardProof,
                    checkpoint_id,
                    ProvingJobCircuitType::GUTATwoGUTA.to_circuit_group_id(),
                    p.nearest_common_ancestor_level as u32,
                    p.nearest_common_ancestor_index as u32,
                    ProvingJobCircuitType::GUTATwoGUTA,
                    ProvingJobDataType::InputWitness,
                    0,
                );

                witnesses.push((w_id, x));
            } else if l_dep_ind != -1 {
                tracing::debug!("Left dependency exists");
                let x = CircuitInputWithDependencies {
                    input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                        checkpoint_tree_root: witnesses[l_dep_ind as usize]
                            .1
                            .input
                            .checkpoint_tree_root,
                        b_checkpoint_tree_root: guta_queue_items.last().as_ref().unwrap().checkpoint_tree_root,
                        stats_a: witnesses[l_dep_ind as usize].1.input.get_combined_stats(),
                        stats_b: guta_queue_items.last().as_ref().unwrap().guta_stats.clone(),
                        nca_proof: res.nca_proofs[i].to_partial(),
                    },
                    dependencies: vec![
                        witnesses[l_dep_ind as usize].0.get_output_id(),
                        guta_queue_items.last().as_ref().unwrap().proof_id,
                    ],
                };
                let w_id = QProvingJobDataID::new(
                    QJobTopic::GenerateStandardProof,
                    checkpoint_id,
                    ProvingJobCircuitType::GUTATwoGUTA.to_circuit_group_id(),
                    p.nearest_common_ancestor_level as u32,
                    p.nearest_common_ancestor_index as u32,
                    ProvingJobCircuitType::GUTATwoGUTA,
                    ProvingJobDataType::InputWitness,
                    0,
                );

                witnesses.push((w_id, x));
            } else {
                panic!("unsupoorted");
            }
        }

        let updates = witnesses
            .iter()
            .map(|(id, w)| KVQPair {
                key: *id,
                value: bincode::serialize(w).unwrap(),
            })
            .collect::<Vec<_>>();

        self.proof_store.set_bytes_by_id_batch(&updates).await?;

        let mut levels = res
            .get_index_levels()
            .iter()
            .map(|l| l.iter().map(|x| witnesses[*x].0).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        let mut guta = GlobalUserTreeAggregatorHeader {
            guta_circuit_whitelist: self.coordinator_config.guta_circuit_whitelist,
            checkpoint_tree_root: guta_queue_items[0].checkpoint_tree_root,
            state_transition: SubTreeNodeStateTransition {
                old_node_value: res.nca_proofs[res.root_proof_index]
                    .old_nearest_common_ancestor_value,
                new_node_value: res.nca_proofs[res.root_proof_index]
                    .new_nearest_common_ancestor_value,
                node_index: F::from_canonical_u64(
                    res.nca_proofs[res.root_proof_index].nearest_common_ancestor_index,
                ),
                node_level: F::from_canonical_u8(
                    res.nca_proofs[res.root_proof_index].nearest_common_ancestor_level,
                ),
            },
            stats: witnesses[res.root_proof_index].1.input.get_combined_stats(),
        };
        tracing::debug!(guta = ?guta, guta_hash = ?guta.qfhash::<QEDHasher>(), "Final GUTA");

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
                ProvingJobCircuitType::GUTAVerifyToCap.to_circuit_group_id(),
                0,
                0,
                ProvingJobCircuitType::GUTAVerifyToCap,
                ProvingJobDataType::InputWitness,
                0,
            );
            self.proof_store
                .set_bytes_by_id(
                    w_id,
                    &bincode::serialize(&w).map_err(|e| anyhow::anyhow!("{:?}", e))?,
                )
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
            tracing::debug!(guta = ?guta, guta_hash = ?guta.qfhash::<QEDHasher>(), "GUTA subtree");
        }

        Ok((levels, guta))
    }

    pub async fn plan_jobs(
        &mut self,
        new_checkpoint_id: u64,
        user_registration_jobs: &Vec<Vec<QProvingJobDataID>>,
        deploy_jobs: &Vec<Vec<QProvingJobDataID>>,
        guta_jobs: &Vec<Vec<QProvingJobDataID>>,
    ) -> anyhow::Result<(QProvingJobDataID,QProvingJobDataID)> {
        let notify_block_complete = QProvingJobDataID::notify_block_complete(new_checkpoint_id);
        let root_state_transition =
            QProvingJobDataID::block_state_transition_input_witness(new_checkpoint_id);
        let root_state_transition_task = QProvingTask::new(&[root_state_transition]);
        let notify_block_complete_task = QProvingTask::new(&[notify_block_complete]);
        self.task_store
            .write_next_tasks(&root_state_transition_task, &notify_block_complete_task)
            .await?;
        let op_agg_group_parts_common_id = 6;
        let state_part_1_common_id = QProvingJobDataID::get_block_aggregate_jobs_group(
            new_checkpoint_id,
            op_agg_group_parts_common_id,
            0,
        );
        let state_part_1_id =
            QProvingJobDataID::block_agg_state_part_1_input_witness(new_checkpoint_id);
        let state_part_1_task = QProvingTask::new(&[state_part_1_id]);
        let state_part_1_common_task = QProvingTask::new(&[state_part_1_common_id]);
        self.task_store
            .write_next_tasks(&state_part_1_common_task, &root_state_transition_task)
            .await?;
        self.task_store
            .write_next_tasks(&state_part_1_task, &state_part_1_common_task)
            .await?;
        let op_agg_group_part_1_id = 11;
        let register_users_agg_job_id = QProvingJobDataID::get_block_aggregate_jobs_group(
            new_checkpoint_id,
            op_agg_group_part_1_id,
            0,
        );
        let deploy_contracts_agg_job_id = QProvingJobDataID::get_block_aggregate_jobs_group(
            new_checkpoint_id,
            op_agg_group_part_1_id,
            1,
        );
        let guta_agg_job_id = QProvingJobDataID::get_block_aggregate_jobs_group(
            new_checkpoint_id,
            op_agg_group_part_1_id,
            2,
        );
        let register_users_agg_task = QProvingTask::new(&[register_users_agg_job_id]);
        let deploy_contracts_agg_task = QProvingTask::new(&[deploy_contracts_agg_job_id]);
        let guta_agg_task = QProvingTask::new(&[guta_agg_job_id]);
        self.task_store.write_next_tasks(&register_users_agg_task, &state_part_1_task).await?;
        self.task_store
            .write_next_tasks(&deploy_contracts_agg_task, &state_part_1_task)
            .await?;
        self.task_store
            .write_next_tasks(&guta_agg_task, &state_part_1_task)
            .await?;
        let user_registration_tasks = user_registration_jobs.iter().map(|jobs| QProvingTask::new(jobs)).collect::<Vec<_>>();
        let deploy_contracts_tasks = deploy_jobs.iter().map(|jobs| QProvingTask::new(jobs)).collect::<Vec<_>>();
        let guta_tasks = guta_jobs.iter().map(|jobs| QProvingTask::new(jobs)).collect::<Vec<_>>();
        self.task_store.write_multidimensional_tasks(&user_registration_tasks, &register_users_agg_task).await?;
        self.task_store.write_multidimensional_tasks(&deploy_contracts_tasks, &deploy_contracts_agg_task).await?;
        self.task_store.write_multidimensional_tasks(&guta_tasks, &guta_agg_task).await?;

        self.task_store.finalize_and_save_topology().await?;

        Ok((state_part_1_id, root_state_transition))
    }

    pub async fn build_block(&mut self) -> anyhow::Result<()> {
        let start = Instant::now();
        info!("coordinator STARTED new block");
        
        self.task_store.clear_task_graph().await?;

        let last_l2_blockstate = self.store.get_latest_l2_block_state().await?;
        let last_user_registration_tree_root = self.store.get_user_registration_tree_root(last_l2_blockstate.checkpoint_id).await?;
        let last_contract_tree_root = self.store.get_contract_tree_root(last_l2_blockstate.checkpoint_id).await?;
        let last_checkpoint_leaf = self.store.get_checkpoint_leaf_data(last_l2_blockstate.checkpoint_id).await?;
        let new_checkpoint_id = last_l2_blockstate.checkpoint_id + 1;

        info!("💥 coordinator processor build block checkpoint_id: {}", new_checkpoint_id);
        let (deploy_jobs, deploy_transition, next_contract_id) =
            self.handle_deploy_contracts(new_checkpoint_id).await?;
        let (
            user_registration_jobs,
            user_registration_transition,
            new_accounts,
            regsitered_users_start_pivot_siblings,
        ) = self.handle_user_registrations(new_checkpoint_id).await?;
        let (guta_jobs, guta_transition) = self.handle_guta_from_realms(new_checkpoint_id).await?;
        let root_deploy_job = deploy_jobs.last()
            .and_then(|jobs| jobs.last())
            .ok_or_else(|| anyhow::anyhow!("No deploy contract jobs found"))?;
        let rooot_user_registration_job = user_registration_jobs.last()
            .and_then(|jobs| jobs.last())
            .ok_or_else(|| anyhow::anyhow!("No user registration jobs found"))?;
        let root_guta_job = guta_jobs.last()
            .and_then(|jobs| jobs.last())
            .ok_or_else(|| anyhow::anyhow!("No GUTA jobs found"))?;

        tracing::info!(new_checkpoint_id = new_checkpoint_id, "Building new checkpoint");
        let (state_part_1_id, root_state_transition) = self.plan_jobs(
            new_checkpoint_id,
            &user_registration_jobs,
            &deploy_jobs,
            &guta_jobs,
        ).await?;

        debug!("Waiting for user registration aggregation job to complete for checkpoint {}", new_checkpoint_id);
        let register_users_proof = self.prover_queue
            .wait_for_job_proof::<C, D>(*rooot_user_registration_job)
            .await?;
        
        debug!("Waiting for deploy contracts aggregation job to complete for checkpoint {}", new_checkpoint_id);
        let deploy_contracts_proof = self.prover_queue
            .wait_for_job_proof::<C, D>(*root_deploy_job)
            .await?;
        
        debug!("Waiting for GUTA aggregation job to complete for checkpoint {}", new_checkpoint_id);
        let guta_proof = self.prover_queue
            .wait_for_job_proof::<C, D>(*root_guta_job)
            .await?;

        let part_1_input = CircuitInputWithDependencies {
            input: QCAggUserRegistartionDeployContractsGUTAInput {
                register_users_state_transition: if user_registration_transition.state_transition_start == QHashOut::ZERO {
                    AggStateTransition {
                        state_transition_start: last_user_registration_tree_root,
                        state_transition_end: last_user_registration_tree_root,
                    }
                }else{
                    user_registration_transition
                },
                deploy_contracts_state_transition: if deploy_transition.state_transition_start == QHashOut::ZERO {
                    AggStateTransition {
                        state_transition_start: last_contract_tree_root,
                        state_transition_end: last_contract_tree_root,
                    }
                }else{
                    deploy_transition
                },
                guta_proof_header: guta_transition,
            },
            dependencies: vec![
                rooot_user_registration_job.get_output_id(),
                root_deploy_job.get_output_id(),
                root_guta_job.get_output_id(),
            ],
        };
        tracing::debug!(part_1_input = ?part_1_input, "Part 1 input for AggUserRegisterDeployContractsGUTA");
        self.proof_store
            .set_bytes_by_id(
                state_part_1_id.get_input_witness_id(),
                &bincode::serialize(&part_1_input).map_err(|e| anyhow::anyhow!("{:?}", e))?,
            )
            .await?;
        
        let register_users_root = {
            let left = QHashOut::try_from(&register_users_proof.public_inputs[0..4])?;
            let right = QHashOut::try_from(&register_users_proof.public_inputs[4..8])?;
            QEDHasher::q_two_to_one(left, right)
        };
        let deploy_contracts_root = {
            let left = QHashOut::try_from(&deploy_contracts_proof.public_inputs[0..4])?;
            let right = QHashOut::try_from(&deploy_contracts_proof.public_inputs[4..8])?;
            QEDHasher::q_two_to_one(left, right)
        };
        let gutas_root = {
            let left = QHashOut::try_from(&guta_proof.public_inputs[0..4])?;
            let right = QHashOut::try_from(&guta_proof.public_inputs[4..8])?;
            QEDHasher::q_two_to_one(left, right)
        };
        
        let pm_rewards_commitment = PMRewardCommitment {
            register_users_root,
            gutas_root,
            deploy_contracts_root,
        };
        
        let partial_input = QCQEDCheckpointStateTransitionInputPartial{
            part_1_header: part_1_input.input,
            old_stats: last_checkpoint_leaf.stats,
            block_time: F::from_canonical_u64(Utc::now().timestamp_millis() as u64),
            final_random_seed_contribution: QHashOut::rand(),
            pm_rewards_commitment,
        };

        tracing::debug!(partial_input = ?partial_input, "Checkpoint state transition partial input");
        let new_checkpoint_leaf = partial_input.get_new_checkpoint_leaf::<QEDHasher>();
        tracing::debug!(new_checkpoint_leaf = ?new_checkpoint_leaf, "New checkpoint leaf");
        let new_checkpoint_leaf_hash= new_checkpoint_leaf.qfhash::<QEDHasher>();
        tracing::debug!(new_checkpoint_leaf_hash = ?new_checkpoint_leaf_hash, "New checkpoint leaf hash");
        let previous_checkpoint_proof = self.store.get_checkpoint_tree_merkle_proof(last_l2_blockstate.checkpoint_id, last_l2_blockstate.checkpoint_id).await?;
        tracing::debug!(previous_checkpoint_proof = ?previous_checkpoint_proof, "Previous checkpoint proof");
        let checkpoint_dmp = self.store.set_checkpoint_tree_leaf_hash_imm(new_checkpoint_id, new_checkpoint_leaf_hash).await?;
        tracing::debug!(checkpoint_dmp = ?checkpoint_dmp, "Checkpoint DMP");
        let checkpoint_tree_update_siblings = checkpoint_dmp.siblings.clone();
        let old_checkpoint_leaf_hash = checkpoint_dmp.old_value;
        let witness_checkpoint_state_transition = CircuitInputWithDependencies{
            input: QCQEDCheckpointStateTransitionInput::<F>{
                partial: partial_input,
                append_checkpoint_tree_proof: checkpoint_dmp,
                previous_checkpoint_proof,
            },
            dependencies: vec![state_part_1_id.get_output_id()],
        };
        tracing::debug!(witness_checkpoint_state_transition = ?witness_checkpoint_state_transition, "Checkpoint state transition witness");
        self.proof_store
            .set_bytes_by_id(
                root_state_transition.get_input_witness_id(),
                &bincode::serialize(&witness_checkpoint_state_transition).map_err(|e| anyhow::anyhow!("{:?}", e))?,
            )
            .await?;

        debug!("Waiting for block proving jobs for checkpoint {}", new_checkpoint_id);
        self.prover_queue
            .wait_for_block_proving_jobs_imm(new_checkpoint_id)
            .await?;
        debug!("Block proving jobs completed for checkpoint {}", new_checkpoint_id);

        let new_l2_block_state = QEDL2BlockState {
            checkpoint_id: last_l2_blockstate.checkpoint_id + 1,
            next_add_withdrawal_id: last_l2_blockstate.next_add_withdrawal_id,
            next_process_withdrawal_id: last_l2_blockstate.next_process_withdrawal_id,
            next_deposit_id: last_l2_blockstate.next_deposit_id,
            total_deposits_claimed_epoch: last_l2_blockstate.total_deposits_claimed_epoch,
            next_user_id: last_l2_blockstate.next_user_id + new_accounts.len() as u64,
            end_balance: last_l2_blockstate.end_balance,
            next_contract_id: next_contract_id,
        };
        tracing::debug!(new_l2_block_state = ?new_l2_block_state, "New L2 block state");

        let lf_state = self.store.get_checkpoint_global_state_roots(new_checkpoint_id).await?;
        self.store
            .set_checkpoint_leaf_data_imm(new_checkpoint_id, &new_checkpoint_leaf)
            .await?;
        self.store
            .set_l2_block_state_imm(&new_l2_block_state)
            .await?;
        let l2_sync = QCheckpointSyncInfoCompact {
            l2_block_state: new_l2_block_state,
            stats: new_checkpoint_leaf.stats,
            state_roots: lf_state,
            checkpoint_tree_update_siblings,
            regsitered_users_start_pivot_siblings,
            registered_users: new_accounts,
            old_checkpoint_leaf_hash,
        };

        tracing::debug!(l2_sync = ?l2_sync, "L2 sync info");
        self.store
            .set_checkpoint_sync_info_imm(l2_sync.clone())
            .await?;
        
        info!("coordinator FINISHED block {} in {}ms", new_l2_block_state.checkpoint_id, start.elapsed().as_millis());
        Ok(())
    }
}
