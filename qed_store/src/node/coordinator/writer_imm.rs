use crate::node::coordinator::{QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm};
use qed_data::{
    config::store_config::{CheckpointSyncInfoTableStore, QEDHasher, UserTreeStore},
    models::{
        checkpoint::sync_info::QEDCheckpointSyncInfoModelCore,
        kvq_merkle::model::KVQFixedConfigMerkleTreeModelCore,
    },
    traits::qdatastore::{
        qmetadata::QMetaDataStoreWriterSync, qtreedata::QTreeDataStoreWriterSync,
    },
};

use async_trait::async_trait;
use plonky2::field::goldilocks_field::GoldilocksField;
use kvq::traits::KVQBinaryStore;
use qed_core::data::qhashout::QHashOut;
use plonky2::field::types::Field;
use qed_data::qdata::contract::QEDContractLeaf;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;
use qed_crypto::hash::{merkle::{
    core::DeltaMerkleProofCore,
    spiderman::SpidermanUpdateProof,
    utils::{
        common::QMerkleNode,
        sub_tree_nca::UpdateNCAProofsWithDependencies,
    },
}, traits::qhashable::QFieldHashable};
use qed_data::{
    protocol::circuit_inputs::{
        agg_part_1::QCAggUserRegistartionDeployContractsGUTAInput,
        checkpoint_transition::QCQEDCheckpointStateTransitionInputPartial,
    },
    guta::{
        header::GlobalUserTreeAggregatorHeader,
        stats::GUTAStats,
    },
    qdata::{
        checkpoint::{QEDCheckpointLeaf, QEDCheckpointLeafStats, QEDL2BlockState},
        contract::ContractCodeDefinition,
        pm_reward_commitment::PMRewardCommitment,
        pm_jobs_completed_stats::PMJobsCompletedStats,
    },
    qsync::coordinator::QEDCheckpointSyncInfoCompact,
};
use qed_crypto::hash::merkle::treeprover::{AggStateTransition, subtree::SubTreeNodeStateTransition};
use chrono::Utc;

type F = GoldilocksField;
#[async_trait]
impl<T: KVQBinaryStore + QEDCoordinatorStoreReaderAsync<F>> QEDCoordinatorStoreWriterAsyncImm<F> for T {
    async fn batch_append_contract_tree_imm(&self, checkpoint_id: u64, start_leaf_index: u64, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>> {

        <Self as QTreeDataStoreWriterSync<F>>::batch_append_contract_tree(
            self,
            checkpoint_id,
            start_leaf_index,
            sub_tree_height,
            leaf_hashes,
        )
    }
    async fn batch_append_user_registration_tree_imm(
        &self,
        checkpoint_id: u64,
        start_leaf_index: u64,
        sub_tree_height: u8,
        leaf_hashes: &[QHashOut<F>],
    ) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>> {
        <Self as QTreeDataStoreWriterSync<F>>::batch_append_user_registration_tree(
            self,
            checkpoint_id,
            start_leaf_index,
            sub_tree_height,
            leaf_hashes,
        )
    }
    async fn injest_user_tree_nodes_imm(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        nodes: &[QMerkleNode<F>],
    ) -> anyhow::Result<UpdateNCAProofsWithDependencies<QHashOut<F>>> {
        UserTreeStore::<Self>::smart_injest_nca_fc(self, root_level, checkpoint_id, nodes)
    }
    async fn set_deposit_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        deposit_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_deposit_tree_leaf_hash(
            self,
            checkpoint_id,
            deposit_id,
            leaf_hash,
        )
    }
    async fn set_withdrawal_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_withdrawal_tree_leaf_hash(
            self,
            checkpoint_id,
            withdrawal_id,
            leaf_hash,
        )
    }
    async fn set_contract_function_whitelist_imm(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        leaves: &[QHashOut<F>],
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_contract_function_whitelist(
            self,
            checkpoint_id,
            contract_id,
            leaves,
        )
    }
    async fn set_contract_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_contract_tree_leaf_hash(
            self,
            checkpoint_id,
            contract_id,
            leaf_hash,
        )
    }
    async fn set_checkpoint_tree_leaf_hash_imm(
        &self,
        checkpoint_id: u64,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_checkpoint_tree_leaf_hash(
            self,
            checkpoint_id,
            leaf_hash,
        )
    }
    async fn set_contract_leaf_data_imm(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        leaf_data: &QEDContractLeaf<F>,
    ) -> anyhow::Result<()> {
        <Self as QMetaDataStoreWriterSync<F>>::set_contract_leaf_data(
            self,
            checkpoint_id,
            contract_id,
            leaf_data,
        )
    }
    async fn set_checkpoint_leaf_data_imm(
        &self,
        checkpoint_id: u64,
        leaf_data: &QEDCheckpointLeaf<F>,
    ) -> anyhow::Result<()> {

        <Self as QMetaDataStoreWriterSync<F>>::set_checkpoint_leaf_data(
            self,
            checkpoint_id,
            leaf_data,
        )
    }
    async fn set_contract_code_definition_imm(
        &self,
        checkpoint_id: u64,
        contract_id: u64,
        definition: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        <Self as QMetaDataStoreWriterSync<F>>::set_contract_code_definition(
            self,
            checkpoint_id,
            contract_id,
            definition,
        )
    }
    async fn set_l2_block_state_imm(&self, block_state: &QEDL2BlockState) -> anyhow::Result<()> {
        <Self as QMetaDataStoreWriterSync<F>>::set_l2_block_state(self, block_state)
    }
    async fn set_checkpoint_sync_info_imm(
        &self,
        sync_info: QEDCheckpointSyncInfoCompact<F>,
    ) -> anyhow::Result<()> {
        CheckpointSyncInfoTableStore::<Self>::set_checkpoint_sync_info(self, sync_info)
    }

    async fn initialize_store(&self, deploy_contracts_root: QHashOut<F>, user_tree_root: QHashOut<F>) -> anyhow::Result<u64> {
        let latest_l2_block_state_or_err = self.get_latest_l2_block_state().await;
        if let Ok(v) = latest_l2_block_state_or_err {
            Ok(v.checkpoint_id)
        } else {
            let genesis_l2_block_state = QEDL2BlockState::get_genesis_value();

            let genesis_checkpoint_stats = QEDCheckpointLeafStats::get_genesis_value();

            let part_1_input = QCAggUserRegistartionDeployContractsGUTAInput {
                register_users_state_transition: AggStateTransition {
                    state_transition_start: QHashOut::ZERO,
                    state_transition_end: QHashOut::ZERO,
                },
                deploy_contracts_state_transition: AggStateTransition {
                    state_transition_start: QHashOut::ZERO,
                    state_transition_end: deploy_contracts_root,
                },
                guta_proof_header: GlobalUserTreeAggregatorHeader {
                    guta_circuit_whitelist: QHashOut::ZERO,
                    checkpoint_tree_root: QHashOut::ZERO,
                    state_transition: SubTreeNodeStateTransition {
                        old_node_value: QHashOut::ZERO,
                        new_node_value: user_tree_root,
                        node_index: F::ZERO,
                        node_level: F::ZERO,
                    },
                    stats: GUTAStats {
                        fees_collected: F::ZERO,
                        user_ops_processed: F::ZERO,
                        total_transactions: F::ZERO,
                        slots_modified: F::ZERO,
                    },
                },
            };

            let pm_rewards_commitment = PMRewardCommitment {
                register_users_root: QHashOut::ZERO,
                gutas_root: QHashOut::ZERO,
                deploy_contracts_root: QHashOut::ZERO,
            };

            let pm_jobs_completed_stats = PMJobsCompletedStats::new_empty();

            let partial_input = QCQEDCheckpointStateTransitionInputPartial {
                part_1_header: part_1_input,
                old_stats: genesis_checkpoint_stats,
                block_time: F::from_canonical_u64(Utc::now().timestamp_millis() as u64),
                final_random_seed_contribution: QHashOut::rand(),
                pm_rewards_commitment,
                pm_jobs_completed: pm_jobs_completed_stats,
            };

            let genesis_checkpoint_leaf = partial_input.get_new_checkpoint_leaf::<QEDHasher>();
            let genesis_global_state_roots = self.get_checkpoint_global_state_roots(1).await?;


            self.set_l2_block_state_imm(&genesis_l2_block_state).await?;
            self.set_checkpoint_leaf_data_imm(0, &genesis_checkpoint_leaf).await?;
            let r = self.set_checkpoint_tree_leaf_hash_imm(0, genesis_checkpoint_leaf.qfhash::<QEDHasher>()).await?;

            let sync_info = QEDCheckpointSyncInfoCompact {
                l2_block_state: genesis_l2_block_state,
                stats:genesis_checkpoint_stats,
                state_roots: genesis_global_state_roots,
                checkpoint_tree_update_siblings: r.siblings.clone(),
                regsitered_users_start_pivot_siblings: vec![],
                registered_users: vec![],
                old_checkpoint_leaf_hash: r.old_value,
                slot: 0,
            };
            self.set_checkpoint_sync_info_imm(sync_info).await?;

            Ok(0)
        }
    }

    async fn set_user_public_key_records(&self, records: &[qed_data::qdata::user_public_key::QEDUserPublicKeyRecord<F>]) -> anyhow::Result<()> {
        use qed_data::config::store_config::UserPublicKeyTableStore;
        use qed_data::models::checkpoint::user_public_keys::QEDUserPublicKeyHelperModelCore;

        UserPublicKeyTableStore::<Self>::set_user_public_key_records(self, records)
    }
}
