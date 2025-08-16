use std::{sync::Arc, time::Instant};
use anyhow::bail;
use kvq::traits::KVQPair;
use plonky2::{
    field::
        types::{Field, PrimeField64}
    ,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use qed_core::{
    config::network_constants::{COORDINATOR_USER_TREE_HEIGHT, COORD_API_GUTA_FROM_REALMS_CHANNEL_ID, CST_USER_UPDATE_CHANNEL_ID, DEFAULT_USER_STATE_TREE_ROOT, GLOBAL_USER_TREE_HEIGHT, REALM_API_GUTA_FROM_USER_CHANNEL_ID, REALM_API_UPDATE_CONTRACT_STATE_TREE_CHANNEL_ID, REALM_USER_TREE_HEIGHT},
    data::qhashout::QHashOut,
    job::{
        drain_queue::CheckpointDrainQueueConsumerAsyncImm,
        history_queue::CheckpointHistoryQueueConsumerAsyncImm,
        id::{QProvingTask, ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID},
        traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
        worker_queue::WorkerEventTransmitterAsyncImm,
    },
};
use qed_crypto::{
    common::{cached_circuit_library::get_cached_circuit_library, circuit_library::CircuitInfoLibraryCore, generic_circuit_verifier::GenericCircuitVerifier, user_id::get_user_id_from_registration_id},
    hash::{merkle::{
            core::{DeltaMerkleProofCore, MerkleProofCore},
            treeprover::{data::CircuitInputWithDependencies, subtree::SubTreeNodeStateTransition},
            utils::common::{QMerkleNode, SimpleMerkleNodeKey},
        }, traits::qhashable::QFieldHashable}
};
use qed_data::{
    guta::{
        api::{GUTARealmCheckpointResult, UserEndCapNonProofCoreInputQueueItem},
        header::GlobalUserTreeAggregatorHeader,
        proof_input::{
            GUTANoChangeFullInput, GUTAOnlyRegisterUsersInput, GUTARegisterUserFullInput, VerifyEndCapSimpleStandardInput, VerifyGUTAToCapCircuitInputSimple, VerifySingleEndCapInput, VerifyTwoEndCapCircuitInput, VerifyTwoGUTAProofGadgetStandardInputSimple
        },
        stats::GUTAStats,
    },
    qdata::{checkpoint::QEDCheckpointLeafCompactWithStateRoots, user::QEDUserLeaf},
    qstore::uct_merkle_nodes::CSTUserUpdate,
};
use qed_data::config::store_config::{QCheckpointSyncInfoCompact, QEDFelt, QEDHasher};
use qed_store::{
    node::realm::{QEDRealmStoreReaderAsync, QEDRealmStoreWriterAsyncImm},
    queue::task_queue::QProvingTaskStore,
};
use serde::{Deserialize, Serialize};
use tracing::info;

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

    pub fn get_standard(rpc_node_id: u32, realm_id: u32) -> Self {
        let library = get_cached_circuit_library::<F>();

        let realm_root_level = COORDINATOR_USER_TREE_HEIGHT;
        let users_per_realm = 1usize << (REALM_USER_TREE_HEIGHT as usize);

        Self {
            rpc_node_id,
            users_per_realm,
            realm_root_level,
            guta_channel_id: REALM_API_GUTA_FROM_USER_CHANNEL_ID + realm_id as u64,
            guta_circuit_whitelist: library
                .get_group_inclusion_proof(
                    ProvingJobCircuitType::GUTATwoGUTA,
                    ProvingJobCircuitType::GUTATwoGUTA,
                )
                .unwrap()
                .root,
            realm_id,
            default_user_state_tree_root: DEFAULT_USER_STATE_TREE_ROOT,
            contract_state_tree_update_channel_id: REALM_API_UPDATE_CONTRACT_STATE_TREE_CHANNEL_ID,
        }
    }
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
    TS: QProvingTaskStore,
> {
    pub store: Arc<SR>,
    pub checkpoint_queue: Arc<DQ>,
    pub sync_queue: Arc<HQ>,
    pub prover_queue: Arc<WQ>,
    pub proof_store: Arc<PS>,
    pub task_store: Arc<TS>,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub realm_config: RealmConfig,
    pub pending_register_users: Vec<MerkleProofCore<QHashOut<F>>>,
}

impl<
        SR: QEDRealmStoreWriterAsyncImm<F> + QEDRealmStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueConsumerAsyncImm,
        HQ: CheckpointHistoryQueueConsumerAsyncImm,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm + QProofStoreWriterAsyncImm + QProofStoreReaderAsync,
        TS: QProvingTaskStore,
    > RealmProcessorContext<SR, DQ, HQ, WQ, PS, TS>
{
    pub async fn new(
        realm_config: RealmConfig,
        store: Arc<SR>,
        checkpoint_queue: Arc<DQ>,
        sync_queue: Arc<HQ>,
        prover_queue: Arc<WQ>,
        proof_store: Arc<PS>,
        task_store: Arc<TS>,
        proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            realm_config,
            store,
            checkpoint_queue,
            sync_queue,
            prover_queue,
            proof_store,
            task_store,
            proof_verifier,
            pending_register_users: Vec::new(),
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

    pub async fn handle_guta_state_updates_from_users(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<()> {
        let updates = self
            .checkpoint_queue
            .cdq_drain_imm::<CSTUserUpdate<QHashOut<F>>>(
                CST_USER_UPDATE_CHANNEL_ID,
            )
            .await?;
        tracing::debug!(checkpoint_id = checkpoint_id, updates_count = updates.len(), "Checkpoint updates");

        self.store.injest_checked_cst_nodes_imm(&updates).await?;

        Ok(())
    }

    pub async fn handle_guta_from_users_ensure_no_topline(
        &self,
        checkpoint_id: u64,
        pending_register_users: &[MerkleProofCore<QHashOut<F>>],
    ) -> anyhow::Result<(
        Vec<Vec<QProvingJobDataID>>,
        GlobalUserTreeAggregatorHeader<F>,
        DeltaMerkleProofCore<QHashOut<F>>,
    )> {
        self.handle_guta_state_updates_from_users(checkpoint_id).await?;

        let (jobs, guta, proof) = self.handle_guta_from_users(checkpoint_id).await?;
        tracing::debug!(guta = ?guta, guta_hash = ?guta.qfhash::<QEDHasher>(), jobs = ?jobs, "Processing GUTA and jobs");
        if pending_register_users.len() != 0 {
            if jobs.len() == 0 {
                tracing::debug!("No jobs to process");
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
                    info!(
                        "DEBUGPRINT[517]: processor.rs:236: pending_register_users={}",
                        serde_json::to_string_pretty(&pending_register_users).unwrap()
                    );
                    tracing::debug!(pending_register_users = ?pending_register_users, "Pending register users");
                    let dmps = self
                        .store
                        .injest_user_leaves_imm(checkpoint_id, COORDINATOR_USER_TREE_HEIGHT, &uleaves)
                        .await?;

                    let regs = dmps
                        .into_iter()
                        .zip(pending_register_users.iter())
                        .map(|(upd, mp)| GUTARegisterUserFullInput {
                            user_registration_tree_merkle_proof: mp.to_owned(),
                            global_user_tree_update_proof: upd,
                        })
                        .collect::<Vec<_>>();
                    tracing::debug!(regs = ?regs, "User registrations");

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
                            node_level: F::from_canonical_u8(COORDINATOR_USER_TREE_HEIGHT),
                        },
                        stats: guta.stats,
                    };
                    tracing::debug!(guta_new = ?guta_new, guta_new_hash = ?guta_new.qfhash::<QEDHasher>(), "New GUTA after user registration");
                    let w = GUTAOnlyRegisterUsersInput {
                        checkpoint_tree_root: guta.checkpoint_tree_root,
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
            tracing::debug!("Processing jobs");
            let last_checkpoint_id = if checkpoint_id == 0 {
                checkpoint_id
            } else {
                checkpoint_id - 1
            };
            tracing::debug!(checkpoint_id = checkpoint_id, last_checkpoint_id = last_checkpoint_id, "Checkpoint IDs");
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
            let input = GUTANoChangeFullInput {
                checkpoint_tree_proof,
                checkpoint_leaf: QEDCheckpointLeafCompactWithStateRoots {
                    checkpoint_leaf: checkpoint_leaf.to_compact::<QEDHasher>(),
                    global_state_roots: roots,
                },
            };
            tracing::debug!(input = ?input, "GUTA no change input");

            let w_id = QProvingJobDataID::core_op_witness(
                ProvingJobCircuitType::GUTANoChange,
                checkpoint_id,
                0,
            );

            self.proof_store
                .set_bytes_by_id(
                    w_id,
                    &bincode::serialize(&input).map_err(|e| anyhow::anyhow!("{:?}", e))?,
                )
                .await?;

            return Ok((vec![vec![w_id]], guta, proof));
        }

        if guta.state_transition.node_level == F::from_canonical_u8(COORDINATOR_USER_TREE_HEIGHT) {
            tracing::debug!("Processing top level GUTA");
            return Ok((jobs, guta, proof));
        } else {
            tracing::debug!("Processing non-top level GUTA");
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
                    COORDINATOR_USER_TREE_HEIGHT,
                    checkpoint_id,
                    (guta.state_transition.node_index.to_canonical_u64())
                        << ((GLOBAL_USER_TREE_HEIGHT as u64
                            - guta.state_transition.node_level.to_canonical_u64())
                            as u64),
                )
                .await?;

            let top_line_siblings_len = guta.state_transition.node_level.to_canonical_u64() as usize - COORDINATOR_USER_TREE_HEIGHT as usize;
            tracing::debug!("Calculated top line siblings length");

            let good_sibs = bp.siblings[(bp.siblings.len() - top_line_siblings_len)..].to_vec();
            tracing::debug!("Extracted good siblings");

            let input = VerifyGUTAToCapCircuitInputSimple {
                guta_proof_header: guta,
                top_line_siblings: good_sibs,
            };
            tracing::debug!(input = ?input, "GUTA to cap input");
            let new_g = input.get_new_guta_header::<QEDHasher>();
            tracing::debug!(new_g = ?new_g, new_g_hash = ?new_g.qfhash::<QEDHasher>(), "New GUTA after to cap");
            let w = CircuitInputWithDependencies::<VerifyGUTAToCapCircuitInputSimple<F>> {
                input,
                dependencies: vec![jobs.last().as_ref().unwrap().last().unwrap().get_output_id()],
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
            tracing::debug!(n_guta = ?n_guta, n_guta_hash = ?n_guta.qfhash::<QEDHasher>(), "New GUTA state");

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

    pub async fn handle_guta_from_users(
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
            )
            .await?;
        tracing::debug!(guta_queue_items = ?guta_queue_items, "GUTA queue items for aggregation");

        if guta_queue_items.len() == 0 {
            tracing::debug!("No GUTA queue items to aggregate");
            let checkpoint_tree_root = self.store.get_latest_checkpoint_tree_root().await?;
            let last_user_tree_root = self
                .store
                .get_user_bottom_tree_merkle_proof(
                    self.realm_config.realm_root_level,
                    checkpoint_id,
                    (self.realm_config.realm_id as u64)<<(REALM_USER_TREE_HEIGHT as u64)
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
            tracing::debug!("Single GUTA queue item");
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
                dependencies: vec![guta_queue_items[0].proof_id.get_output_id()],
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
                    COORDINATOR_USER_TREE_HEIGHT,
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
            tracing::debug!(r = ?r, single = ?single, "Single GUTA processing");

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
            .injest_user_tree_nodes_imm(checkpoint_id, COORDINATOR_USER_TREE_HEIGHT, &mnu)
            .await?;
        tracing::debug!(res = ?res, "GUTA aggregation result");

        let mut updates = Vec::with_capacity(res.nca_proofs.len());
        let mut combo_stats = Vec::with_capacity(res.nca_proofs.len());

        let checkpoint_tree_root = guta_queue_items[0].checkpoint_tree_proof.root;
        for (i, p) in res.nca_proofs.iter().enumerate() {
            let (l_dep_ind, r_dep_ind) = res.dependencies[i];
            if l_dep_ind == -1 && r_dep_ind == -1 {
                tracing::debug!("Both GUTA dependencies are new");
                let x = CircuitInputWithDependencies {
                    input: VerifyTwoEndCapCircuitInput {
                        guta_circuit_whitelist: self.realm_config.guta_circuit_whitelist,
                        a_end_cap: guta_queue_items[i * 2].get_verify_end_cap_simple_input(),
                        b_end_cap: guta_queue_items[i * 2 + 1].get_verify_end_cap_simple_input(),
                        nca_proof: res.nca_proofs[i].to_partial(),
                    },
                    dependencies: vec![
                        guta_queue_items[i * 2].proof_id.get_output_id(),
                        guta_queue_items[i * 2 + 1].proof_id.get_output_id(),
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
                tracing::debug!("Both GUTA dependencies exist");
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

                let a_checkpoint_tree_root = bincode::deserialize::<CircuitInputWithDependencies<VerifyTwoGUTAProofGadgetStandardInputSimple<F>>>(&updates[l_dep_ind as usize].value)?.input.checkpoint_tree_root;
                let b_checkpoint_tree_root = bincode::deserialize::<CircuitInputWithDependencies<VerifyTwoGUTAProofGadgetStandardInputSimple<F>>>(&updates[r_dep_ind as usize].value)?.input.checkpoint_tree_root;
                let x = CircuitInputWithDependencies {
                    input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                        checkpoint_tree_root: a_checkpoint_tree_root,
                        b_checkpoint_tree_root,
                        stats_a: l_stats,
                        stats_b: r_stats,
                        nca_proof: res.nca_proofs[i].to_partial(),
                    },
                    dependencies: vec![l_proof_id.get_output_id(), r_proof_id.get_output_id()],
                };

                updates.push(KVQPair {
                    key: w_id,
                    value: bincode::serialize(&x)?,
                });
            } else if l_dep_ind != -1 {
                tracing::debug!("Left GUTA dependency exists");
                let (l_proof_id, l_stats) = combo_stats[l_dep_ind as usize];
                let a_checkpoint_tree_root = bincode::deserialize::<CircuitInputWithDependencies<VerifyTwoGUTAProofGadgetStandardInputSimple<F>>>(&updates[l_dep_ind as usize].value)?.input.checkpoint_tree_root;
                let x = CircuitInputWithDependencies {
                    input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                        checkpoint_tree_root: a_checkpoint_tree_root,
                        b_checkpoint_tree_root: guta_queue_items.last().as_ref().unwrap().checkpoint_tree_proof.root,
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
                        l_proof_id.get_output_id(),
                        guta_queue_items.last().as_ref().unwrap().proof_id.get_output_id(),
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

        tracing::debug!(guta = ?guta, guta_hash = ?guta.qfhash::<QEDHasher>(), "Final aggregated GUTA");

        Ok((levels, guta, res.link_proof))
    }

    pub async fn plan_jobs(
        &mut self,
        new_checkpoint_id: u64,
        guta_jobs: &Vec<Vec<QProvingJobDataID>>,
        finished_job: QProvingJobDataID,
    ) -> anyhow::Result<()> {
        let guta_tasks = guta_jobs.iter().map(|jobs| QProvingTask::new(jobs)).collect::<Vec<_>>();
        let finished_job_task = QProvingTask::new(&[finished_job]);
        self.task_store.write_multidimensional_tasks(&guta_tasks, &finished_job_task).await?;
        
        self.task_store.finalize_and_save_topology().await?;

        let task_graph = self.task_store.get_task_graph().await;
        self.task_store.save_job_dependency_graph(&task_graph, new_checkpoint_id).await
            .map_err(|e| anyhow::anyhow!("Failed to save job dependency graph for checkpoint {}: {}", new_checkpoint_id, e))?;
        tracing::info!("Saved realm job dependency graph for checkpoint {}", new_checkpoint_id);
        
        Ok(())
    }

    pub async fn build_block(&mut self) -> anyhow::Result<()> {
        let start = Instant::now();
        info!("realm STARTED new block");
        self.task_store.clear_task_graph().await?;
        let last_l2_blockstate = self.store.get_latest_l2_block_state().await?;
        let new_checkpoint_id = last_l2_blockstate.checkpoint_id+1;
        info!("🔔 realm processor build block checkpoint_id: {}", new_checkpoint_id);
        let has_tasks = self.has_pending_tasks(new_checkpoint_id).await?;
        if !has_tasks {
            bail!("No pending tasks for checkpoint {}, skipping block construction", new_checkpoint_id);
        }
        // todo fix the bug!!!
        let pending_users = if self.pending_register_users.len() > 32 {
            self.pending_register_users.split_off(self.pending_register_users.len() - 32)
        }else{
            let q = self.pending_register_users.clone();
            self.pending_register_users = vec![];
            q
        };
        let (guta_jobs, guta_transition, guta_dmp) = self.handle_guta_from_users_ensure_no_topline(new_checkpoint_id, &pending_users).await?;
        println!("guta_jobs: {:?}",guta_jobs);
        let finished_job = QProvingJobDataID::notify_realm_complete(new_checkpoint_id, self.realm_config.realm_id);
        let res = GUTARealmCheckpointResult{
            checkpoint_id: new_checkpoint_id,
            guta_stats: guta_transition.stats,
            top_line_proof: guta_dmp,
            checkpoint_tree_root: guta_transition.checkpoint_tree_root,
            proof_id: **(guta_jobs.last().as_ref().unwrap().last().as_ref().unwrap()),
        };
        self.proof_store.set_bytes_by_id(finished_job, &bincode::serialize(&res).map_err(|e| anyhow::anyhow!("{:?}",e))?).await?;
        // Plan the job dependency graph
        self.plan_jobs(new_checkpoint_id, &guta_jobs, finished_job).await?;

        info!("realm FINISHED new block {} in {}ms",new_checkpoint_id, start.elapsed().as_millis());
        Ok(())
    }

    /// Check if there are pending tasks for the given checkpoint
    pub async fn has_pending_tasks(&self, checkpoint_id: u64) -> anyhow::Result<bool> {
        // Check if there are pending user registrations in the internal list
        if !self.pending_register_users.is_empty() {
            info!("Found {} pending user registrations in internal list", self.pending_register_users.len());
            return Ok(true);
        }

        // Check if there are user operations in the GUTA queue
        let guta_queue_items = self
            .checkpoint_queue
            .cdq_peek_imm::<UserEndCapNonProofCoreInputQueueItem<F>>(
                self.realm_config.guta_channel_id,
            )
            .await?;

        if !guta_queue_items.is_empty() {
            info!("Found {} pending GUTA queue items", guta_queue_items.len());
            return Ok(true);
        }

        // Check for contract state updates
        let cst_updates = self
            .checkpoint_queue
            .cdq_peek_imm::<CSTUserUpdate<QHashOut<F>>>(
                CST_USER_UPDATE_CHANNEL_ID,
            )
            .await?;

        if !cst_updates.is_empty() {
            info!("Found {} pending contract state updates", cst_updates.len());
            return Ok(true);
        }

        info!("No pending tasks found for checkpoint {}", checkpoint_id);
        Ok(false)
    }
}
