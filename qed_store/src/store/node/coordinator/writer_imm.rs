use crate::{
    config::store_config::{CheckpointSyncInfoTableStore, QEDHasher, UserTreeStore},
    models::{
        checkpoint::sync_info::{self, QEDCheckpointSyncInfoModelCore},
        kvq_merkle::model::KVQFixedConfigMerkleTreeModelCoreImmutable,
    },
    node::coordinator::store_traits::{QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm},
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
        checkpoint::{QEDCheckpointLeaf, QEDCheckpointLeafStats, QEDL2BlockState},
        contract::{ContractCodeDefinition, QEDContractLeaf},
    },
    qsync::coordinator::QEDCheckpointSyncInfoCompact,
};
type F = GoldilocksField;
#[async_trait]
impl<T: QEDStorageAdapterImmutable + Send + Sync + QEDCoordinatorStoreReaderAsync<F>> QEDCoordinatorStoreWriterAsyncImm<F> for T {
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
    async fn initialize_store(&self) -> anyhow::Result<u64> {

        let latest_l2_block_state_or_err = self.get_latest_l2_block_state().await;
        if latest_l2_block_state_or_err.is_ok() {
            let v = latest_l2_block_state_or_err.unwrap();
            Ok(v.checkpoint_id)
        }else{
            // database not initialized with data for the genesis block

            let genesis_l2_block_state = QEDL2BlockState::get_genesis_value();

            let genesis_checkpoint_stats = QEDCheckpointLeafStats::get_genesis_value();
            let stats_hash = genesis_checkpoint_stats.qfhash::<QEDHasher>();
            let genesis_global_state_roots = self.get_checkpoint_global_state_roots(1).await?;
            let genesis_checkpoint_leaf = QEDCheckpointLeaf{
                global_chain_root: genesis_global_state_roots.qfhash::<QEDHasher>(),
                stats: genesis_checkpoint_stats,
            };


            println!("genesis_stats_hash: {:?} ({})",stats_hash, serde_json::to_string_pretty(&stats_hash).unwrap());

            println!("genesis_global_state_roots: {}",serde_json::to_string_pretty(&genesis_global_state_roots).unwrap());
            println!("genesis_checkpoint_leaf: {}",serde_json::to_string_pretty(&genesis_checkpoint_leaf).unwrap());

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
                registered_users_secp256k1_public_keys: vec![],
            };
            self.set_checkpoint_sync_info_imm(sync_info).await?;

            Ok(0)

        }

    }
}
