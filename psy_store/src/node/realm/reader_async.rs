use async_trait::async_trait;
use kvq::traits::KVQBinaryStore;
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_core::{config::network_constants::GLOBAL_USER_TREE_HEIGHT, data::qhashout::QHashOut};
use psy_crypto::hash::merkle::core::MerkleProofCore;
use psy_data::{
    config::store_config::{CheckpointSyncInfoTableStore, UserPublicKeyTableStore, UserTreeStore},
    models::{
        checkpoint::{sync_info::PsyCheckpointSyncInfoModelReaderCore, user_public_keys::PsyUserPublicKeyHelperModelReaderCore},
        kvq_merkle::model::KVQFixedConfigMerkleTreeModelReaderCore,
    },
    qdata::{
        checkpoint::{PsyBlockState, PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf},
        user::PsyUserLeaf,
    },
    traits::qdatastore::{qmetadata::QMetaDataStoreReaderSync, qtreedata::QTreeDataStoreReaderSync},
};

use crate::node::realm::PsyRealmStoreReaderAsync;

type F = GoldilocksField;

// #[cfg(feature = "is_sync")]
#[async_trait]
impl<T: KVQBinaryStore> PsyRealmStoreReaderAsync<F> for T {
    async fn get_first_user_id(&self, public_key: QHashOut<F>) -> anyhow::Result<u64> {
        Ok(
            UserPublicKeyTableStore::<Self>::get_first_user_for_public_key_hash_if_exists(self, public_key)?
                .ok_or(anyhow::anyhow!("User not found".to_string()))?
                .user_id,
        )
    }
    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<PsyCheckpointLeaf<F>> {
        <Self as QMetaDataStoreReaderSync<F>>::get_checkpoint_leaf_data(self, checkpoint_id).await
    }
    async fn get_latest_block_state(&self) -> anyhow::Result<PsyBlockState> {
        Ok(CheckpointSyncInfoTableStore::<Self>::get_latest_checkpoint_sync_info_compact(self)?.block_state)
    }

    async fn get_block_state(&self, checkpoint_id: u64) -> anyhow::Result<PsyBlockState> {
        Ok(CheckpointSyncInfoTableStore::<Self>::get_checkpoint_sync_info_compact_or_latest(self, checkpoint_id)?.block_state)
    }

    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        Ok(
            CheckpointSyncInfoTableStore::<Self>::get_checkpoint_sync_info_compact_or_latest(self, checkpoint_id)?
                .state_roots
                .user_registration_tree_root,
        )
    }
    async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_latest_checkpoint_tree_root(self).await
    }
    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_checkpoint_tree_root(self, checkpoint_id).await
    }
    async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_checkpoint_tree_leaf_hash(self, checkpoint_id, leaf_checkpoint_id).await
    }
    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_checkpoint_tree_merkle_proof(self, checkpoint_id, leaf_checkpoint_id).await
    }

    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<PsyCheckpointGlobalStateRoots<F>> {
        Ok(CheckpointSyncInfoTableStore::<Self>::get_checkpoint_sync_info_compact_or_latest(self, checkpoint_id)?.state_roots)
    }
    async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<PsyUserLeaf<F>> {
        <Self as QMetaDataStoreReaderSync<F>>::get_user_leaf_data(self, checkpoint_id, user_id).await
    }
    async fn get_user_contract_state_tree_root(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_state_tree_root(self, checkpoint_id, user_id, contract_id).await
    }
    async fn get_user_contract_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_state_tree_leaf_hash(self, checkpoint_id, user_id, contract_id, height, leaf_id)
            .await
    }
    async fn get_user_contract_state_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_state_tree_merkle_proof(self, checkpoint_id, user_id, contract_id, height, leaf_id)
            .await
    }
    async fn get_user_contract_tree_root(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_tree_root(self, checkpoint_id, user_id).await
    }
    async fn get_user_contract_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_tree_leaf_hash(self, checkpoint_id, user_id, contract_id).await
    }
    async fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_contract_tree_merkle_proof(self, checkpoint_id, user_id, contract_id).await
    }
    async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        Ok(
            CheckpointSyncInfoTableStore::<Self>::get_checkpoint_sync_info_compact_or_latest(self, checkpoint_id)?
                .state_roots
                .user_tree_root,
        )
    }
    async fn get_user_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_tree_leaf_hash(self, checkpoint_id, user_id).await
    }
    async fn get_user_bottom_tree_merkle_proof(
        &self,
        root_level: u8,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserTreeStore::<Self>::get_sub_tree_proof_fc(self, checkpoint_id, root_level, GLOBAL_USER_TREE_HEIGHT, user_id)
    }
    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserTreeStore::<Self>::get_sub_tree_proof_fc(self, checkpoint_id, root_level, leaf_level, leaf_index)
    }

    async fn get_user_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_tree_merkle_proof(self, checkpoint_id, user_id).await
    }
}
