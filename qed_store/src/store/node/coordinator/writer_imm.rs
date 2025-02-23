use crate::{
    config::store_config::{CheckpointSyncInfoTableStore, QEDHasher, UserTreeStore},
    models::{
        checkpoint::sync_info::QEDCheckpointSyncInfoModelCore,
        kvq_merkle::model::KVQFixedConfigMerkleTreeModelCoreImmutable,
    },
    node::coordinator::store_traits::QEDCoordinatorStoreWriterAsyncImm,
    store::imm::core::QEDStorageAdapterImmutable,
    traits::qdatastore::{
        qmetadata::QMetaDataStoreWriterSync, qtreedata::QTreeDataStoreWriterSync,
    },
};

use async_trait::async_trait;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::{merkle::{
    core::DeltaMerkleProofCore,
    spiderman::SpidermanUpdateProof,
    utils::{
        common::QMerkleNode,
        sub_tree_nca::UpdateNCAProofsWithDependencies,
    },
}, traits::qhashable::QFieldHashable};
use qed_data::{
    qdata::{
        checkpoint::{QEDCheckpointLeaf, QEDL2BlockState},
        contract::{ContractCodeDefinition, QEDContractLeaf},
    },
    qsync::coordinator::QEDCheckpointSyncInfoCompact,
};
type F = GoldilocksField;
#[async_trait]
impl<T: QEDStorageAdapterImmutable + Send + Sync> QEDCoordinatorStoreWriterAsyncImm<F> for T {
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
    async fn batch_append_user_registration_tree_f_imm(
        &self,
        checkpoint_id: F,
        start_leaf_index: F,
        sub_tree_height: u8,
        leaf_hashes: &[QHashOut<F>],
    ) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>> {
        <Self as QTreeDataStoreWriterSync<F>>::batch_append_user_registration_tree_f(
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
        UserTreeStore::smart_injest_nca_fc_imm(self, root_level, checkpoint_id, nodes)
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
    async fn set_deposit_tree_leaf_hash_f_imm(
        &self,
        checkpoint_id: F,
        deposit_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_deposit_tree_leaf_hash_f(
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
    async fn set_withdrawal_tree_leaf_hash_f_imm(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_withdrawal_tree_leaf_hash_f(
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
    async fn set_contract_function_whitelist_f_imm(
        &self,
        checkpoint_id: F,
        contract_id: F,
        leaves: &[QHashOut<F>],
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_contract_function_whitelist_f(
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
    async fn set_contract_tree_leaf_hash_f_imm(
        &self,
        checkpoint_id: F,
        contract_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreWriterSync<F>>::set_contract_tree_leaf_hash_f(
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
    async fn set_checkpoint_tree_leaf_hash_f_imm(
        &self,
        checkpoint_id: F,
        leaf_hash: QHashOut<F>,
    ) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {

        <Self as QTreeDataStoreWriterSync<F>>::set_checkpoint_tree_leaf_hash_f(
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
    async fn set_contract_leaf_data_f_imm(
        &self,
        checkpoint_id: F,
        contract_id: F,
        leaf_data: &QEDContractLeaf<F>,
    ) -> anyhow::Result<()> {
        <Self as QMetaDataStoreWriterSync<F>>::set_contract_leaf_data_f(
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
    async fn set_checkpoint_leaf_data_f_imm(
        &self,
        checkpoint_id: F,
        leaf_data: &QEDCheckpointLeaf<F>,
    ) -> anyhow::Result<()> {

        <Self as QMetaDataStoreWriterSync<F>>::set_checkpoint_leaf_data_f(
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
    async fn set_contract_code_definition_f_imm(
        &self,
        checkpoint_id: F,
        contract_id: F,
        definition: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        <Self as QMetaDataStoreWriterSync<F>>::set_contract_code_definition_f(
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
    async fn commit_block(&self, _checkpoint_id: u64) -> anyhow::Result<()> {
        todo!()
        //<Self as QMetaDataStoreWriterSync<F>>::commit_block(self, checkpoint_id)
    }
}
