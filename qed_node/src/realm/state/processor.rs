use std::sync::Arc;

use kvq::traits::KVQPair;
use plonky2::{
    field::{
        packed::PackedField,
        types::{Field, PrimeField64},
    },
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use qed_core::{
    config::network_constants::{GLOBAL_USER_TREE_HEIGHT, REALM_USER_TREE_HEIGHT},
    data::qhashout::QHashOut,
    job::{
        drain_queue::CheckpointDrainQueueConsumerAsyncImm,
        history_queue::CheckpointHistoryQueueConsumerAsyncImm,
        id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID},
        traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
        worker_queue::WorkerEventTransmitterAsyncImm,
    },
};
use qed_crypto::{
    common::generic_circuit_verifier::GenericCircuitVerifier,
    hash::{
        merkle::{
            core::{DeltaMerkleProofCore, MerkleProofCore},
            treeprover::{data::CircuitInputWithDependencies, subtree::SubTreeNodeStateTransition},
            utils::common::{QMerkleNode, SimpleMerkleNodeKey},
        },
        traits::{hasher::FieldQHasher, qhashable::QFieldHashable},
    },
};
use qed_data::{
    guta::{
        api::UserEndCapNonProofCoreInputQueueItem,
        header::GlobalUserTreeAggregatorHeader,
        proof_input::{
            GUTAOnlyRegisterUsersInput, GUTARegisterUserFullInput, VerifyEndCapSimpleStandardInput,
            VerifyGUTAToCapCircuitInputSimple, VerifySingleEndCapInput,
            VerifyTwoEndCapCircuitInput, VerifyTwoGUTAProofGadgetStandardInputSimple,
        },
        stats::GUTAStats,
    },
    qdata::{checkpoint::QEDL2BlockState, user::QEDUserLeaf},
    qstore::uct_merkle_nodes::CSTUserUpdate,
};
use qed_store::{
    config::store_config::{QCheckpointSyncInfoCompact, QEDFelt, QEDHasher},
    node::realm::{QEDRealmStoreReaderAsync, QEDRealmStoreWriterAsyncImm},
    store::node::realm::writer_imm::get_user_id_from_registration_id,
};
use serde::{Deserialize, Serialize};

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RealmConfig {
    pub rpc_node_id: u32,
    pub realm_id: u32,
    pub users_per_realm: usize,
    pub realm_root_level: u8,

    pub guta_channel_id: u64,
    pub guta_circuit_whitelist: QHashOut<F>,
    pub default_user_state_tree_root: QHashOut<F>,

    pub contract_state_tree_update_channel_id: u64,
}

impl RealmConfig {
    pub fn includes_user_id(&self, id: u64) -> bool {
        let r64 = self.realm_id as u64;
        id >= r64 * (self.users_per_realm as u64) && id < (r64 + 1) * (self.users_per_realm as u64)
    }
    pub fn get_local_user_id_masked(&self, global_user_id: u64) -> u64 {
        global_user_id & ((1u64 << (self.realm_root_level as u64)) - 1u64)
    }
}
#[derive(Clone)]
pub struct RealmProcessorContext<
    SR: QEDRealmStoreWriterAsyncImm<F> + QEDRealmStoreReaderAsync<F>,
    DQ: CheckpointDrainQueueConsumerAsyncImm,
    HQ: CheckpointHistoryQueueConsumerAsyncImm,
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
    pub realm_config: RealmConfig,
    pub pending_register_users: Vec<MerkleProofCore<QHashOut<F>>>,
    chkpoint_id: u64,
    //pub checkpoint_id: u64,
    //pub end_cap_verifier_data: VerifierOnlyCircuitData<C, D>,
}

impl<
        SR: QEDRealmStoreWriterAsyncImm<F> + QEDRealmStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueConsumerAsyncImm,
        HQ: CheckpointHistoryQueueConsumerAsyncImm,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm + QProofStoreWriterAsyncImm + QProofStoreReaderAsync,
    > RealmProcessorContext<SR, DQ, HQ, WQ, PS>
{
    pub async fn new(
        realm_config: RealmConfig,
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
            realm_config,
            store,
            checkpoint_queue,
            sync_queue,
            prover_queue,
            proof_store,
            proof_verifier,
            latest_block_state,
            chkpoint_id: checkpoint_id,
            pending_register_users: Vec::new(),
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

    pub async fn handle_checkpoint_sync(
        &mut self,
        input: QCheckpointSyncInfoCompact,
    ) -> anyhow::Result<()> {
        let dmps = input.get_registered_user_merkle_proofs::<QEDHasher>();
        self.store
            .injest_checkpoint_sync_data_imm(input.to_sync_info::<QEDHasher>())
            .await?;
        dmps.into_iter().for_each(|x| {
            let real_id = get_user_id_from_registration_id(x.index);

            if self.realm_config.includes_user_id(real_id) {
                self.pending_register_users.push(x);
            }
        });

        Ok(())
    }
    pub async fn handle_guta_state_updates_from_realms(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<()> {
        let updates = self
            .checkpoint_queue
            .cdq_drain_imm::<CSTUserUpdate<QHashOut<F>>>(
                self.realm_config.guta_channel_id,
                checkpoint_id,
            )
            .await?;

        self.store.injest_checked_cst_nodes_imm(&updates).await?;

        Ok(())
    }

    pub async fn handle_guta_from_realms_ensure_no_topline(
        &self,
        checkpoint_id: u64,
        pending_register_users: &[MerkleProofCore<QHashOut<F>>],
    ) -> anyhow::Result<(
        Vec<Vec<QProvingJobDataID>>,
        GlobalUserTreeAggregatorHeader<F>,
        DeltaMerkleProofCore<QHashOut<F>>,
    )> {
        let (jobs, guta, proof) = self.handle_guta_from_realms(checkpoint_id).await?;
        if pending_register_users.len() != 0 {
            if jobs.len() == 0 {
                //assert!(pending_register_users.len() <= 64, "we do not support batches of more than 64 register users for ");
                if pending_register_users.len() <= 64 {
                    let uleaves = pending_register_users
                        .iter()
                        .map(|x| QEDUserLeaf {
                            public_key: x.value,
                            user_state_tree_root: self.realm_config.default_user_state_tree_root,
                            balance: F::ZERO,
                            nonce: F::ZERO,
                            last_checkpoint_id: F::ZERO,
                            event_index: F::ZERO,
                            user_id: F::from_noncanonical_u64(get_user_id_from_registration_id(
                                x.index,
                            )),
                        })
                        .collect::<Vec<_>>();
                    let dmps = self
                        .store
                        .injest_user_leaves_imm(checkpoint_id, REALM_USER_TREE_HEIGHT, &uleaves)
                        .await?;

                    let regs = dmps
                        .into_iter()
                        .zip(pending_register_users.iter())
                        .map(|(upd, mp)| GUTARegisterUserFullInput {
                            user_registration_tree_merkle_proof: mp.to_owned(),
                            global_user_tree_update_proof: upd,
                        })
                        .collect::<Vec<_>>();

                    let guta_new = GlobalUserTreeAggregatorHeader {
                        checkpoint_tree_root: guta.checkpoint_tree_root,
                        guta_circuit_whitelist: guta.guta_circuit_whitelist,
                        state_transition: SubTreeNodeStateTransition {
                            old_node_value: regs[0].global_user_tree_update_proof.old_root,
                            new_node_value: regs
                                .last()
                                .as_ref()
                                .unwrap()
                                .global_user_tree_update_proof
                                .new_root,
                            node_index: F::from_canonical_u32(self.realm_config.realm_id),
                            node_level: F::from_canonical_u8(REALM_USER_TREE_HEIGHT),
                        },
                        stats: guta.stats,
                    };
                    let w = GUTAOnlyRegisterUsersInput {
                        checkpoint_tree_root: self
                            .store
                            .get_checkpoint_tree_root(checkpoint_id)
                            .await?,
                        guta_register_user_inputs: regs,
                    };
                    let w_id = QProvingJobDataID::new(
                        QJobTopic::GenerateStandardProof,
                        checkpoint_id,
                        ProvingJobCircuitType::GUTAOnlyRegisterUsers.to_circuit_group_id(),
                        0,
                        0,
                        ProvingJobCircuitType::GUTAOnlyRegisterUsers,
                        ProvingJobDataType::InputWitness,
                        0,
                    );

                    self.proof_store
                        .set_bytes_by_id(
                            w_id,
                            &bincode::serialize(&w).map_err(|e| anyhow::anyhow!("{:?}", e))?,
                        )
                        .await?;

                    return Ok((
                        vec![vec![w_id.get_output_id()]],
                        guta_new,
                        DeltaMerkleProofCore::single_value(
                            self.realm_config.realm_id as u64,
                            guta_new.state_transition.old_node_value,
                            guta_new.state_transition.new_node_value,
                        ),
                    ));
                } else {
                    todo!("support more than 64 users in empty reg");
                }
            } else {
                assert!(jobs.len() <= 32, "currently only supports one 32 job batch");
                todo!("working on it")
            }
        }

        if jobs.len() == 0 {
            return Ok((jobs, guta, proof));
        }

        if guta.state_transition.node_level == F::from_canonical_u8(REALM_USER_TREE_HEIGHT) {
            return Ok((jobs, guta, proof));
        } else {
            // add a job to verify to the root cap
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

            let bp = self
                .store
                .get_user_bottom_tree_merkle_proof(
                    REALM_USER_TREE_HEIGHT,
                    checkpoint_id,
                    (guta.state_transition.node_index.to_canonical_u64())
                        << ((GLOBAL_USER_TREE_HEIGHT as u64
                            - guta.state_transition.node_level.to_canonical_u64())
                            as u64),
                )
                .await?;

            let top_line_siblings_len = REALM_USER_TREE_HEIGHT as usize
                - guta.state_transition.node_level.to_canonical_u64() as usize;

            let good_sibs = bp.siblings[(bp.siblings.len() - top_line_siblings_len)..].to_vec();

            let w = CircuitInputWithDependencies::<VerifyGUTAToCapCircuitInputSimple<F>> {
                input: VerifyGUTAToCapCircuitInputSimple {
                    guta_proof_header: guta,
                    top_line_siblings: good_sibs,
                },
                dependencies: vec![*jobs.last().as_ref().unwrap().last().unwrap()],
            };

            self.proof_store
                .set_bytes_by_id(
                    w_id,
                    &bincode::serialize(&w).map_err(|e| anyhow::anyhow!("{:?}", e))?,
                )
                .await?;

            let mut n_jobs = jobs;

            n_jobs.push(vec![w_id]);

            let n_guta = w.input.get_new_guta_header::<QEDHasher>();

            return Ok((
                n_jobs,
                n_guta,
                DeltaMerkleProofCore::single_value(
                    self.realm_config.realm_id as u64,
                    n_guta.state_transition.old_node_value,
                    n_guta.state_transition.new_node_value,
                ),
            ));
        }
    }

    pub async fn handle_guta_from_realms(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<(
        Vec<Vec<QProvingJobDataID>>,
        GlobalUserTreeAggregatorHeader<F>,
        DeltaMerkleProofCore<QHashOut<F>>,
    )> {
        let guta_queue_items = self
            .checkpoint_queue
            .cdq_drain_imm::<UserEndCapNonProofCoreInputQueueItem<F>>(
                self.realm_config.guta_channel_id,
                checkpoint_id,
            )
            .await?;
        if guta_queue_items.len() == 0 {
            let checkpoint_tree_root = self.store.get_latest_checkpoint_tree_root().await?;
            let last_user_tree_root = self
                .store
                .get_user_bottom_tree_merkle_proof(
                    self.realm_config.realm_root_level,
                    checkpoint_id,
                    0,
                )
                .await?
                .root;

            return Ok((
                Vec::new(),
                GlobalUserTreeAggregatorHeader {
                    guta_circuit_whitelist: self.realm_config.guta_circuit_whitelist,
                    checkpoint_tree_root,
                    state_transition: SubTreeNodeStateTransition {
                        old_node_value: last_user_tree_root,
                        new_node_value: last_user_tree_root,
                        node_index: F::from_canonical_u32(self.realm_config.realm_id),
                        node_level: F::from_canonical_u8(self.realm_config.realm_root_level),
                    },
                    stats: GUTAStats::default(),
                },
                DeltaMerkleProofCore::single_value(
                    self.realm_config.realm_id as u64,
                    last_user_tree_root,
                    last_user_tree_root,
                ),
            ));
        } else if guta_queue_items.len() == 1 {
            let single = CircuitInputWithDependencies::<VerifySingleEndCapInput<F>> {
                input: VerifySingleEndCapInput {
                    guta_circuit_whitelist: self.realm_config.guta_circuit_whitelist,
                    a_end_cap: VerifyEndCapSimpleStandardInput {
                        guta_stats: guta_queue_items[0].input.stats,
                        checkpoint_root: guta_queue_items[0].checkpoint_tree_proof.root,
                        checkpoint_historical_merkle_proof: guta_queue_items[0]
                            .checkpoint_tree_proof
                            .clone(),
                    },
                    start_user_leaf_hash: guta_queue_items[0]
                        .input
                        .state_transition
                        .start_user_leaf_hash,
                    end_user_leaf_hash: guta_queue_items[0]
                        .input
                        .state_transition
                        .end_user_leaf_hash,
                    user_id: guta_queue_items[0].input.state_transition.user_id,
                },
                dependencies: vec![guta_queue_items[0].proof_id],
            };
            self.store
                .injest_user_leaves_batch_imm(
                    checkpoint_id,
                    &[guta_queue_items[0].input.new_user_leaf],
                )
                .await?;
            let r = self
                .store
                .injest_user_tree_nodes_imm(
                    checkpoint_id,
                    REALM_USER_TREE_HEIGHT,
                    &[QMerkleNode {
                        key: SimpleMerkleNodeKey {
                            index: guta_queue_items[0]
                                .input
                                .state_transition
                                .user_id
                                .to_canonical_u64(),
                            level: GLOBAL_USER_TREE_HEIGHT,
                        },
                        value: guta_queue_items[0]
                            .input
                            .state_transition
                            .end_user_leaf_hash,
                    }],
                )
                .await?;

            let id = QProvingJobDataID::core_op_witness(
                ProvingJobCircuitType::GUTASingleEndCap,
                checkpoint_id,
                0,
            );

            self.proof_store
                .set_bytes_by_id(id.get_input_witness_id(), &bincode::serialize(&single)?)
                .await?;
            return Ok((
                vec![vec![id]],
                single.input.get_guta_header_a(),
                r.link_proof,
            ));
        }

        // start real stuff

        let mnu = guta_queue_items
            .iter()
            .map(|x| QMerkleNode {
                key: SimpleMerkleNodeKey {
                    index: x.input.state_transition.user_id.to_canonical_u64(),
                    level: GLOBAL_USER_TREE_HEIGHT,
                },
                value: x.input.state_transition.end_user_leaf_hash,
            })
            .collect::<Vec<_>>();

        self.store
            .injest_user_leaves_batch_imm(
                checkpoint_id,
                &guta_queue_items
                    .iter()
                    .map(|x| x.input.new_user_leaf)
                    .collect::<Vec<_>>(),
            )
            .await?;

        let res = self
            .store
            .injest_user_tree_nodes_imm(checkpoint_id, REALM_USER_TREE_HEIGHT, &mnu)
            .await?;

        let mut updates = Vec::with_capacity(res.nca_proofs.len());
        let mut combo_stats = Vec::with_capacity(res.nca_proofs.len());

        let checkpoint_tree_root = guta_queue_items[0].checkpoint_tree_proof.root;
        for (i, p) in res.nca_proofs.iter().enumerate() {
            let (l_dep_ind, r_dep_ind) = res.dependencies[i];
            if l_dep_ind == -1 && r_dep_ind == -1 {
                let x = CircuitInputWithDependencies {
                    input: VerifyTwoEndCapCircuitInput {
                        guta_circuit_whitelist: self.realm_config.guta_circuit_whitelist,
                        a_end_cap: guta_queue_items[i * 2].get_verify_end_cap_simple_input(),
                        b_end_cap: guta_queue_items[i * 2 + 1].get_verify_end_cap_simple_input(),
                        nca_proof: res.nca_proofs[i].to_partial(),
                    },
                    dependencies: vec![
                        guta_queue_items[i * 2].proof_id,
                        guta_queue_items[i * 2 + 1].proof_id,
                    ],
                };
                let w_id = QProvingJobDataID::new(
                    QJobTopic::GenerateStandardProof,
                    checkpoint_id,
                    ProvingJobCircuitType::GUTATwoEndCap.to_circuit_group_id(),
                    p.nearest_common_ancestor_level as u32,
                    p.nearest_common_ancestor_index as u32,
                    ProvingJobCircuitType::GUTATwoEndCap,
                    ProvingJobDataType::InputWitness,
                    0,
                );

                combo_stats.push((
                    w_id.get_output_id(),
                    guta_queue_items[i * 2]
                        .input
                        .stats
                        .combine_with(&guta_queue_items[i * 2 + 1].input.stats),
                ));

                updates.push(KVQPair {
                    key: w_id,
                    value: bincode::serialize(&x)?,
                });
            } else if r_dep_ind != -1 && l_dep_ind != -1 {
                let (l_proof_id, l_stats) = combo_stats[l_dep_ind as usize];
                let (r_proof_id, r_stats) = combo_stats[r_dep_ind as usize];

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
                combo_stats.push((w_id.get_output_id(), l_stats.combine_with(&r_stats)));
                let x = CircuitInputWithDependencies {
                    input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                        checkpoint_tree_root,
                        stats_a: l_stats,
                        stats_b: r_stats,
                        nca_proof: res.nca_proofs[i].to_partial(),
                    },
                    dependencies: vec![l_proof_id, r_proof_id],
                };

                updates.push(KVQPair {
                    key: w_id,
                    value: bincode::serialize(&x)?,
                });
            } else if l_dep_ind != -1 {
                let (l_proof_id, l_stats) = combo_stats[l_dep_ind as usize];

                let x = CircuitInputWithDependencies {
                    input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                        checkpoint_tree_root,
                        stats_a: l_stats,
                        stats_b: guta_queue_items
                            .last()
                            .as_ref()
                            .unwrap()
                            .input
                            .stats
                            .clone(),
                        nca_proof: res.nca_proofs[i].to_partial(),
                    },
                    dependencies: vec![
                        l_proof_id,
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
                combo_stats.push((w_id.get_output_id(), l_stats.combine_with(&x.input.stats_b)));

                updates.push(KVQPair {
                    key: w_id,
                    value: bincode::serialize(&x)?,
                });
            } else {
                panic!("unsupoorted");
            }
        }

        self.proof_store.set_bytes_by_id_batch(&updates).await?;

        let levels = res
            .get_index_levels()
            .iter()
            .map(|l| l.iter().map(|x| combo_stats[*x].0).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        let guta = GlobalUserTreeAggregatorHeader {
            guta_circuit_whitelist: self.realm_config.guta_circuit_whitelist,
            checkpoint_tree_root: checkpoint_tree_root,
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
            stats: combo_stats[res.root_proof_index].1,
        };

        Ok((levels, guta, res.link_proof))
    }
}
