use kvq::traits::KVQBinaryStore;
use qed_data::{
    config::store_config::{CheckpointSyncInfoTableStore, UserTreeStore},
    models::{
        checkpoint::sync_info::QEDCheckpointSyncInfoModelReaderCore,
        kvq_merkle::model::
            KVQFixedConfigMerkleTreeModelReaderCore
        ,
    },
    traits::qdatastore::{
        qmetadata::QMetaDataStoreReaderSync, qtreedata::QTreeDataStoreReaderSync,
    },
};
use crate::node::realm::QEDRealmStoreReaderAsync;

use async_trait::async_trait;
use plonky2::field::{goldilocks_field::GoldilocksField, types::PrimeField64};
use qed_core::{config::network_constants::GLOBAL_USER_TREE_HEIGHT, data::qhashout::QHashOut};
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_data::qdata::{
    checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState},
    user::QEDUserLeaf,
};

type F = GoldilocksField;

#[cfg(feature = "is_sync")]
#[async_trait]
impl<T: KVQBinaryStore + Send + Sync> QEDRealmStoreReaderAsync<F> for T {
    async fn get_checkpoint_leaf_data(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        <Self as QMetaDataStoreReaderSync<F>>::get_checkpoint_leaf_data(self, checkpoint_id)
    }
    async fn get_checkpoint_leaf_data_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        <Self as QMetaDataStoreReaderSync<F>>::get_checkpoint_leaf_data_f(self, checkpoint_id)
    }
    async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState> {
        Ok(
            CheckpointSyncInfoTableStore::<Self>::get_latest_checkpoint_sync_info_compact(
                self,
            )?
            .l2_block_state,
        )
        }

    async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState> {
        Ok(
            CheckpointSyncInfoTableStore::<Self>::get_checkpoint_sync_info_compact_or_latest(
                self,
                checkpoint_id,
            )?
            .l2_block_state,
        )
    }
    async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState> {
        Ok(
            CheckpointSyncInfoTableStore::<Self>::get_checkpoint_sync_info_compact_or_latest(
                self,
                checkpoint_id.to_canonical_u64(),
            )?
            .l2_block_state,
        )
    }

    async fn get_user_registration_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        Ok(
            CheckpointSyncInfoTableStore::<Self>::get_checkpoint_sync_info_compact_or_latest(
                self,
                checkpoint_id,
            )?
            .state_roots
            .user_registration_tree_root,
        )
    }
    async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_latest_checkpoint_tree_root(self)
    }
    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_checkpoint_tree_root(self, checkpoint_id)
    }
    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_checkpoint_tree_root_f(self, checkpoint_id)
    }
    async fn get_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_checkpoint_tree_leaf_hash(
            self,
            checkpoint_id,
            leaf_checkpoint_id,
        )
    }
    async fn get_checkpoint_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_checkpoint_tree_leaf_hash_f(
            self,
            checkpoint_id,
            leaf_checkpoint_id,
        )
    }
    async fn get_checkpoint_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_checkpoint_tree_merkle_proof(
            self,
            checkpoint_id,
            leaf_checkpoint_id,
        )
    }
    async fn get_checkpoint_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_checkpoint_tree_merkle_proof_f(
            self,
            checkpoint_id,
            leaf_checkpoint_id,
        )
    }

    async fn get_checkpoint_global_state_roots(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QEDCheckpointGlobalStateRoots<F>> {
        Ok(
            CheckpointSyncInfoTableStore::<Self>::get_checkpoint_sync_info_compact_or_latest(
                self,
                checkpoint_id,
            )?
            .state_roots,
        )
    }
    async fn get_user_leaf_data(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QEDUserLeaf<F>> {
        <Self as QMetaDataStoreReaderSync<F>>::get_user_leaf_data(self, checkpoint_id, user_id)
    }
    async fn get_user_leaf_data_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<QEDUserLeaf<F>> {
        <Self as QMetaDataStoreReaderSync<F>>::get_user_leaf_data_f(self, checkpoint_id, user_id)
    }
    async fn get_user_contract_state_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_state_tree_root(
            self,
            checkpoint_id,
            user_id,
            contract_id,
        )
    }
    async fn get_user_contract_state_tree_root_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_state_tree_root_f(
            self,
            checkpoint_id,
            user_id,
            contract_id,
        )
    }
    async fn get_user_contract_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_state_tree_leaf_hash(
            self,
            checkpoint_id,
            user_id,
            contract_id,
            height,
            leaf_id,
        )
    }
    async fn get_user_contract_state_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_state_tree_leaf_hash_f(
            self,
            checkpoint_id,
            user_id,
            contract_id,
            height,
            leaf_id,
        )
    }
    async fn get_user_contract_state_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_state_tree_merkle_proof(
            self,
            checkpoint_id,
            user_id,
            contract_id,
            height,
            leaf_id,
        )
    }
    async fn get_user_contract_state_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_state_tree_merkle_proof_f(
            self,
            checkpoint_id,
            user_id,
            contract_id,
            height,
            leaf_id,
        )
    }
    async fn get_user_contract_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_tree_root(
            self,
            checkpoint_id,
            user_id,
        )
    }
    async fn get_user_contract_tree_root_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_tree_root_f(
            self,
            checkpoint_id,
            user_id,
        )
    }
    async fn get_user_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_tree_leaf_hash(
            self,
            checkpoint_id,
            user_id,
            contract_id,
        )
    }
    async fn get_user_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_tree_leaf_hash_f(
            self,
            checkpoint_id,
            user_id,
            contract_id,
        )
    }
    async fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_tree_merkle_proof(
            self,
            checkpoint_id,
            user_id,
            contract_id,
        )
    }
    async fn get_user_contract_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_tree_merkle_proof_f(
            self,
            checkpoint_id,
            user_id,
            contract_id,
        )
    }
    async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        Ok(
            CheckpointSyncInfoTableStore::<Self>::get_checkpoint_sync_info_compact_or_latest(
                self,
                checkpoint_id,
            )?
            .state_roots
            .user_tree_root,
        )
    }
    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_tree_root_f(self, checkpoint_id)
    }
    async fn get_user_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_tree_leaf_hash(self, checkpoint_id, user_id)
    }
    async fn get_user_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_tree_leaf_hash_f(
            self,
            checkpoint_id,
            user_id,
        )
    }
    async fn get_user_bottom_tree_merkle_proof(
        &self,
        root_level: u8,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserTreeStore::<Self>::get_sub_tree_proof_fc(
            self,
            checkpoint_id,
            root_level,
            GLOBAL_USER_TREE_HEIGHT,
            user_id,
        )
    }
    async fn get_user_bottom_tree_merkle_proof_f(
        &self,
        root_level: u8,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserTreeStore::<Self>::get_sub_tree_proof_fc(
            self,
            checkpoint_id.to_canonical_u64(),
            root_level,
            GLOBAL_USER_TREE_HEIGHT,
            user_id.to_canonical_u64(),
        )
    }
    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserTreeStore::<Self>::get_sub_tree_proof_fc(
            self,
            checkpoint_id,
            root_level,
            leaf_level,
            leaf_index,
        )
    }

    async fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_tree_merkle_proof(
            self,
            checkpoint_id,
            user_id,
        )
    }

    async fn get_user_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_tree_merkle_proof_f(
            self,
            checkpoint_id,
            user_id,
        )
    }
}
