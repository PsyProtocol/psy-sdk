use std::sync::Arc;

use chrono::Utc;
use kvq::traits::KVQPair;
use plonky2::{
    field::types::Field,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use qed_core::{
    config::network_constants::{
        BATCH_DEPLOY_CONTRACT_SUB_TREE_HEIGHT, BATCH_USER_REGISTRAITION_MAX_SUB_TREES, BATCH_USER_REGISTRAITION_SUB_TREE_HEIGHT, COORD_API_DEPLOY_CONTRACT_CHANNEL_ID, COORD_API_GUTA_FROM_REALMS_CHANNEL_ID, COORD_API_REGISTER_USER_CHANNEL_ID, REALM_USER_TREE_HEIGHT
    },
    data::qhashout::QHashOut,
    job::{
        drain_queue::{CheckpointDrainQueueConsumerAsyncImm, WithDrainQueueMetadata},
        history_queue::CheckpointHistoryQueueEmitterAsyncImm,
        id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID},
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
        traits::qhashable::QFieldHashable,
    },
    signature::zk::data::ZKPublicKeyInfo,
};
use qed_data::{
    guta::{
        api::SubmitGUTARealmResultAPIQueueItem,
        header::GlobalUserTreeAggregatorHeader,
        proof_input::{
            GUTANoChangeFullInput, VerifyGUTAToCapCircuitInputSimple,
            VerifyTwoGUTAProofGadgetStandardInputSimple,
        },
        stats::GUTAStats,
    },
    proof_store::builder::ProofStoreBuilder,
    protocol::circuit_inputs::{agg_part_1::QCAggUserRegistartionDeployContractsGUTAInput, checkpoint_transition::{QCQEDCheckpointStateTransitionInput, QCQEDCheckpointStateTransitionInputPartial}},
    qblock::cmds::deploy_contract::QBCDeployContractWithRoot,
    qdata::{
        checkpoint::{QEDCheckpointLeafCompactWithStateRoots, QEDL2BlockState},
        contract::QEDContractLeaf,

    },
};
use qed_store::{
    config::store_config::{QCheckpointSyncInfoCompact, QEDFelt, QEDHasher}, node::coordinator::store_traits::{
        QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm,
    }
};
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::coordinator::state::user_map::{get_node_redis_pool, save_user_mapping_to_redis};


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

        let realm_root_level = REALM_USER_TREE_HEIGHT;
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
> {
    pub store: Arc<SR>,
    pub checkpoint_queue: Arc<DQ>,
    pub sync_queue: Arc<HQ>,
    pub prover_queue: Arc<WQ>,
    pub proof_store: Arc<PS>,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub latest_block_state: QEDL2BlockState,
    pub coordinator_config: CoordinatorConfig,
    chkpoint_id: u64,
    //pub checkpoint_id: u64,
    //pub end_cap_verifier_data: VerifierOnlyCircuitData<C, D>,
}

impl<
        SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueConsumerAsyncImm,
        HQ: CheckpointHistoryQueueEmitterAsyncImm,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm,
    > CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS>
{
    pub async fn new(
        coordinator_config: CoordinatorConfig,
        store: Arc<SR>,
        checkpoint_queue: Arc<DQ>,
        sync_queue: Arc<HQ>,
        prover_queue: Arc<WQ>,
        proof_store: Arc<PS>,
        proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    ) -> anyhow::Result<Self> {
        let latest_block_state: QEDL2BlockState = store.get_latest_l2_block_state().await?;
        let checkpoint_id = latest_block_state.checkpoint_id;

        Ok(Self {
            coordinator_config,
            store,
            checkpoint_queue,
            prover_queue,
            sync_queue,
            proof_store,
            proof_verifier,
            latest_block_state,
            chkpoint_id: checkpoint_id,
            //checkpoint_id,
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
        //Vec<ZKPublicKeyInfo<F>>,
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
            //user_registrations,
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
        eprintln!("DEBUGPRINT[724]: processor.rs:326: user_registrations={}", serde_json::to_string_pretty(&user_registrations).unwrap());

        let start_user_id = last_l2_blockstate.next_user_id;

        let redis_pool = get_node_redis_pool()?;

        for (i, pubkey_info) in user_registrations.iter().enumerate() {
            let register_id = start_user_id + i as u64;
            let user_id = get_user_id_from_registration_id(register_id);
            let redis_pool = redis_pool.clone();

            match save_user_mapping_to_redis(&redis_pool, user_id, pubkey_info).await {
                Ok(_) => {
                    tracing::info!("✅ Saved user_id={} mapping to Redis", user_id);
                }
                Err(e) => {
                    tracing::error!("❌ Failed to save user_id={} to Redis: {:?}", user_id, e);
                    return Err(e);
                }
            }
        }

        let new_public_keys = user_registrations
            .iter()
            .map(|x| x.to_hash::<QEDHasher>())
            .collect::<Vec<_>>();


        let mut psb = ProofStoreBuilder::new();
        let wits = self
            .store
            .batch_append_user_registration_tree_imm(
                checkpoint_id,
                start_user_id,
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
                .get_user_registration_tree_merkle_proof(checkpoint_id, start_user_id)
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
        eprintln!("DEBUGPRINT[537]: processor.rs:399: checkpoint_id={}", checkpoint_id);
        let mut guta_queue_items = self
            .checkpoint_queue
            .cdq_drain_imm::<SubmitGUTARealmResultAPIQueueItem<F>>(
                self.coordinator_config.guta_channel_id,
                checkpoint_id,
            )
            .await?;
        eprintln!("DEBUGPRINT[530]: processor.rs:406: guta_queue_items={}", serde_json::to_string_pretty(&guta_queue_items).unwrap());

        if guta_queue_items.len() == 0 {
            eprintln!("DEBUGPRINT[531]: processor.rs:408 (after if guta_queue_items.len() == 0 )");
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
            eprintln!("DEBUGPRINT[526]: processor.rs:442: guta_header={}", serde_json::to_string_pretty(&guta_header).unwrap());
            eprintln!("DEBUGPRINT[526]: processor.rs:442: guta_header_hash={}", serde_json::to_string_pretty(&guta_header.qfhash::<QEDHasher>()).unwrap());
            let input = GUTANoChangeFullInput {
                checkpoint_tree_proof,
                checkpoint_leaf: QEDCheckpointLeafCompactWithStateRoots {
                    checkpoint_leaf: checkpoint_leaf.to_compact::<QEDHasher>(),
                    global_state_roots: roots,
                },
            };
            eprintln!("DEBUGPRINT[532]: processor.rs:452: input={}", serde_json::to_string_pretty(&input).unwrap());

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
            eprintln!("DEBUGPRINT[533]: processor.rs:465 (after  else if guta_queue_items.len() == 1 )");
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
            eprintln!("DEBUGPRINT[534]: processor.rs:500: rw={}", serde_json::to_string_pretty(&rw).unwrap());
            let r_with_deps = CircuitInputWithDependencies {
                input: rw,
                dependencies: vec![guta_queue_items[0].proof_id],
            };
            eprintln!("DEBUGPRINT[556]: processor.rs:507: guta_queue_items[0]={}", serde_json::to_string_pretty(&guta_queue_items[0]).unwrap());

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
            eprintln!("DEBUGPRINT[557]: processor.rs:536: res={}", serde_json::to_string_pretty(&res).unwrap());
        //for local testing conv
        let good_old = self.store.get_user_top_tree_cap_root(checkpoint_id-1, res.nearest_common_ancestor_level, res.nearest_common_ancestor_index).await?;
        eprintln!("DEBUGPRINT[564]: processor.rs:538: good_old={}", good_old);

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
        eprintln!("DEBUGPRINT[675]: processor.rs:582: guta_queue_items={}", serde_json::to_string_pretty(&guta_queue_items).unwrap());

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
        eprintln!("DEBUGPRINT[640]: processor.rs:604: res={}", serde_json::to_string_pretty(&res).unwrap());
        /*
        res.nca_proofs.iter().enumerate().map(|(i, p)| {
            let (left_dep_ind, right_dep_ind ) = res.dependencies[i];


        })*/

        let mut witnesses = Vec::with_capacity(res.nca_proofs.len());

        for (i, p) in res.nca_proofs.iter().enumerate() {
            eprintln!("DEBUGPRINT[667]: processor.rs:615: res.nca_proofs[i].verify()={:#?}", res.nca_proofs[i].verify::<QEDHasher>());
            let (l_dep_ind, r_dep_ind) = res.dependencies[i];
            if l_dep_ind == -1 && r_dep_ind == -1 {
                eprintln!("DEBUGPRINT[560]: processor.rs:589 (after if l_dep_ind == -1 && r_dep_ind == -1 )");
                let input = VerifyTwoGUTAProofGadgetStandardInputSimple {
                    checkpoint_tree_root: guta_queue_items[i * 2].checkpoint_tree_root,
                    stats_a: guta_queue_items[i * 2].guta_stats,
                    stats_b: guta_queue_items[i * 2 + 1].guta_stats,
                    nca_proof: res.nca_proofs[i].to_partial(),
                };
                tracing::info!("❗guta input: {:?}", input);

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
                eprintln!("DEBUGPRINT[559]: processor.rs:614 (after  else if r_dep_ind != -1 && l_dep_ind !=…)");
                let x = CircuitInputWithDependencies {
                    input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                        checkpoint_tree_root: witnesses[l_dep_ind as usize]
                            .1
                            .input
                            .checkpoint_tree_root,
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
                eprintln!("DEBUGPRINT[558]: processor.rs:642 (after  else if l_dep_ind != -1 )");
                let x = CircuitInputWithDependencies {
                    input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                        checkpoint_tree_root: witnesses[l_dep_ind as usize]
                            .1
                            .input
                            .checkpoint_tree_root,
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
        eprintln!("DEBUGPRINT[535]: processor.rs:704: guta={}", serde_json::to_string_pretty(&guta).unwrap());
        eprintln!("DEBUGPRINT[664]: processor.rs:739: guta.qhash::<QEDHasher>()={}", guta.qfhash::<QEDHasher>());

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
            eprintln!("DEBUGPRINT[527]: processor.rs:737: guta={}", serde_json::to_string_pretty(&guta).unwrap());
            eprintln!("DEBUGPRINT[527]: processor.rs:737: guta_hash={}", serde_json::to_string_pretty(&guta.qfhash::<QEDHasher>()).unwrap());
        }

        /*self.proof_store.set_bytes_by_id(input_id, &bincode::serialize(&queue_item).map_err(|e| anyhow::anyhow!("{:?}",e))?).await?;

        let d = WithDrainQueueMetadata::<QProvingJobDataID>::new_params(
            self.guta_channel_id,
            checkpoint_id,
            queue_item.realm_id,
            input_id,
        );*/

        Ok((levels, guta))
    }

    pub async fn build_block(&self) -> anyhow::Result<()> {
        let last_l2_blockstate = self.store.get_latest_l2_block_state().await?;
        let last_user_registration_tree_root = self.store.get_user_registration_tree_root(last_l2_blockstate.checkpoint_id).await?;
        let last_contract_tree_root = self.store.get_contract_tree_root(last_l2_blockstate.checkpoint_id).await?;
        //let last_user_tree_root = self.store.get_user_tree_root(last_l2_blockstate.checkpoint_id).await?;

        //let state_roots = self.store.get_checkpoint_global_state_roots(last_l2_blockstate.checkpoint_id).await?;
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

        println!("new_checkpoint_id: {}",new_checkpoint_id);

        let notify_block_complete = QProvingJobDataID::notify_block_complete(new_checkpoint_id);
        let root_state_transition =
            QProvingJobDataID::block_state_transition_input_witness(new_checkpoint_id);

        self.proof_store
            .write_next_jobs(&[root_state_transition], &[notify_block_complete])
            .await?;

        let op_agg_group_parts_common_id = 6;

        let state_part_1_common_id = QProvingJobDataID::get_block_aggregate_jobs_group(
            new_checkpoint_id,
            op_agg_group_parts_common_id,
            0,
        );
/* 
        let state_part_2_common_id = QProvingJobDataID::get_block_aggregate_jobs_group(
            new_checkpoint_id,
            op_agg_group_parts_common_id,
            1,
        );
        */

        let state_part_1_id =
            QProvingJobDataID::block_agg_state_part_1_input_witness(new_checkpoint_id);
        //let state_part_2_id =QProvingJobDataID::block_agg_state_part_2_input_witness(new_checkpoint_id);

        self.proof_store
            .write_next_jobs(
                &[state_part_1_common_id],//, state_part_2_common_id],
                &[root_state_transition],
            )
            .await?;

        self.proof_store
            .write_next_jobs(&[state_part_1_id], &[state_part_1_common_id])
            .await?;
        //self.proof_store  .write_next_jobs(&[state_part_2_id], &[state_part_2_common_id]).await?;



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

        self.proof_store
            .write_next_jobs(
                &[
                    register_users_agg_job_id,
                    deploy_contracts_agg_job_id,
                    guta_agg_job_id,
                ],
                &[state_part_1_id],
            )
            .await?;
        self.proof_store
            .write_multidimensional_jobs(&user_registration_jobs, &[register_users_agg_job_id])
            .await?;
        self.proof_store
            .write_multidimensional_jobs(&deploy_jobs, &[deploy_contracts_agg_job_id])
            .await?;
        self.proof_store
            .write_multidimensional_jobs(&guta_jobs, &[guta_agg_job_id])
            .await?;
        //let new_user_tree_root = self.store.get_user_tree_root(last_l2_blockstate.checkpoint_id).await?;
        /*let user_agg = AggStateTransition {
            state_transition_start: last_user_tree_root,
            state_transition_end: new_user_tree_root,
        };
        let user_agg_alt = AggStateTransition {
            state_transition_start: guta_transition.state_transition.old_node_value,
            state_transition_end: guta_transition.state_transition.new_node_value,
        };*/
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
                user_registration_jobs
                    .last()
                    .as_ref()
                    .unwrap()
                    .last()
                    .unwrap().get_output_id(),
                deploy_jobs.last().as_ref().unwrap().last().unwrap().get_output_id(),
                guta_jobs.last().as_ref().unwrap().last().unwrap().get_output_id(),
            ],
        };
        eprintln!("DEBUGPRINT[727]: processor.rs:939: part_1_input={}", serde_json::to_string_pretty(&part_1_input).unwrap());

        self.proof_store
            .set_bytes_by_id(
                state_part_1_id.get_input_witness_id(),
                &bincode::serialize(&part_1_input).map_err(|e| anyhow::anyhow!("{:?}", e))?,
            )
            .await?;

        /*QEDCheckpointGlobalStateRoots {
            contract_tree_root: deploy_transition.state_transition_end,
            deposit_tree_root,
            user_tree_root: guta_transition.state_transition.new_node_value,
            withdrawal_tree_root,
            user_registration_tree_root: user_registration_transition.state_transition_end,
        };*/
        let partial_input = QCQEDCheckpointStateTransitionInputPartial{
                part_1_header: part_1_input.input,
                old_stats: last_checkpoint_leaf.stats,
                block_time: F::from_canonical_u64(Utc::now().timestamp_millis() as u64),
                final_random_seed_contribution: QHashOut::rand(),
        };
        eprintln!("DEBUGPRINT[728]: processor.rs:955: partial_input={}", serde_json::to_string_pretty(&partial_input).unwrap());

        //let old_checkpoint_leaf = partial_input.get_old_checkpoint_leaf::<QEDHasher>();
        let new_checkpoint_leaf = partial_input.get_new_checkpoint_leaf::<QEDHasher>();
        eprintln!("DEBUGPRINT[592]: processor.rs:929: new_checkpoint_leaf={}", serde_json::to_string_pretty(&new_checkpoint_leaf).unwrap());
        let new_checkpoint_leaf_hash= new_checkpoint_leaf.qfhash::<QEDHasher>();
        eprintln!("DEBUGPRINT[593]: processor.rs:931: new_checkpoint_leaf_hash={}", serde_json::to_string_pretty(&new_checkpoint_leaf_hash).unwrap());
        //let old_checkpoint_leaf_hash= old_checkpoint_leaf.qfhash::<QEDHasher>();

        /*println!("stateroottss: {}, {:?}",serde_json::to_string_pretty(&state_roots).unwrap(), state_roots.qfhash::<QEDHasher>());

        println!("got: get_old_state_roots: {}, {:?}",serde_json::to_string_pretty(&partial_input.get_old_state_roots::<QEDHasher>()).unwrap(), partial_input.get_old_state_roots::<QEDHasher>().qfhash::<QEDHasher>());
        println!("[{}] 1ostr: {:?}",new_checkpoint_id, old_checkpoint_leaf.global_chain_root);
        println!("[{}] 1leafo: {:?}",new_checkpoint_id, old_checkpoint_leaf_hash);
        println!("[{}] 1nstr: {:?}",new_checkpoint_id, new_checkpoint_leaf.global_chain_root);
        println!("[{}] 1leafn: {:?}",new_checkpoint_id, new_checkpoint_leaf_hash);
        */
        let previous_checkpoint_proof = self.store.get_checkpoint_tree_merkle_proof(last_l2_blockstate.checkpoint_id, last_l2_blockstate.checkpoint_id).await?;
        eprintln!("DEBUGPRINT[595]: processor.rs:943: previous_checkpoint_proof={}", serde_json::to_string_pretty(&previous_checkpoint_proof).unwrap());
        //println!("last_chpk_leaf_hash: {:?}, {}",last_checkpoint_leaf.qfhash::<QEDHasher>(), serde_json::to_string_pretty(&last_checkpoint_leaf).unwrap());
        //println!("previous_checkpoint_proof[{}]: {:?}", previous_checkpoint_proof.index, previous_checkpoint_proof.value);

        let checkpoint_dmp = self.store.set_checkpoint_tree_leaf_hash_imm(new_checkpoint_id, new_checkpoint_leaf_hash).await?;
        eprintln!("DEBUGPRINT[594]: processor.rs:947: checkpoint_dmp={}", serde_json::to_string_pretty(&checkpoint_dmp).unwrap());
        
        
        let checkpoint_tree_update_siblings = checkpoint_dmp.siblings.clone();
        let witness_checkpoint_state_transition = CircuitInputWithDependencies{
            input: QCQEDCheckpointStateTransitionInput::<F>{
                partial: partial_input,
                append_checkpoint_tree_proof: checkpoint_dmp,
                previous_checkpoint_proof,
            },
            dependencies: vec![state_part_1_id.get_output_id()],
        };
        eprintln!("DEBUGPRINT[589]: processor.rs:957: witness_checkpoint_state_transition={}", serde_json::to_string_pretty(&witness_checkpoint_state_transition).unwrap());


        self.proof_store
            .set_bytes_by_id(
                root_state_transition.get_input_witness_id(),
                &bincode::serialize(&witness_checkpoint_state_transition).map_err(|e| anyhow::anyhow!("{:?}", e))?,
            )
            .await?;



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
        eprintln!("DEBUGPRINT[590]: processor.rs:979: new_l2_block_state={}", serde_json::to_string_pretty(&new_l2_block_state).unwrap());
        self.prover_queue
            .enqueue_jobs_imm(
                &([
                    guta_jobs[0].to_vec(),
                    deploy_jobs[0].to_vec(),
                    user_registration_jobs[0].to_vec(),
                ]
                .concat()),
            )
            .await?;
            let lf_state = self.store.get_checkpoint_global_state_roots(new_checkpoint_id).await?;
            //println!("set new leaf: {:#?}\n\nnew_leaf_hash {}: {:?},\nlf: {:?}, {:?}",new_checkpoint_leaf,new_checkpoint_id,new_checkpoint_leaf.qfhash::<QEDHasher>(),lf_state,lf_state.qfhash::<QEDHasher>());
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
        };
        eprintln!("DEBUGPRINT[591]: processor.rs:1007: l2_sync={}", serde_json::to_string_pretty(&l2_sync).unwrap());
        self.store
            .set_checkpoint_sync_info_imm(l2_sync.clone())
            .await?;

        //todo! mark, should commit the txn
        self.sync_queue.chq_push_imm(l2_sync).await?;

        tracing::info!(
            "lastest block state: {:?}",
            self.store.get_latest_l2_block_state().await?,
        );

        Ok(())
    }

    pub async fn commit_block(&self,checkpoint_id: u64) -> anyhow::Result<()> {
        self.store.commit_block(checkpoint_id).await
    }

    pub async fn rollback_block(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        self.store.rollback_block(checkpoint_id).await
    }
}

