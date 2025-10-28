use async_trait::async_trait;
use kvq::traits::KVQBinaryStore;
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_core::{config::network_constants::GLOBAL_USER_TREE_HEIGHT, data::qhashout::QHashOut};
use psy_crypto::hash::merkle::core::MerkleProofCore;
use psy_data::{
    config::store_config::{CheckpointSyncInfoTableStore, RealmStatusTableStore, UserPublicKeyTableStore, UserTreeStore},
    models::{
        checkpoint::{sync_info::PsyCheckpointSyncInfoModelReaderCore, user_public_keys::PsyUserPublicKeyHelperModelReaderCore},
        kvq_merkle::model::{KVQFixedConfigMerkleTreeModelReaderCore, KVQMerkleTreeModelReaderCore},
        realm_status::RealmStatusModelReaderCore,
    },
    qdata::{
        checkpoint::{PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf, PsyBlockState},
        contract::{ContractCodeDefinition, PsyContractLeaf},
        realm_status::BasicRealmStatus,
    },
    qsync::coordinator::PsyCheckpointSyncInfoCompact,
    traits::qdatastore::{qmetadata::QMetaDataStoreReaderSync, qtreedata::QTreeDataStoreReaderSync},
};
use tracing::info;

use crate::node::coordinator::PsyCoordinatorStoreReaderAsync;
type F = GoldilocksField;

// #[cfg(feature = "is_sync")]
#[async_trait]
impl<T: KVQBinaryStore> PsyCoordinatorStoreReaderAsync<F> for T {
    async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<PsyContractLeaf<F>> {
        <Self as QMetaDataStoreReaderSync<F>>::get_contract_leaf_data(self, contract_id).await
    }

    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<PsyCheckpointLeaf<F>> {
        <Self as QMetaDataStoreReaderSync<F>>::get_checkpoint_leaf_data(self, checkpoint_id).await
    }

    async fn get_contract_code_definition(&self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition> {
        <Self as QMetaDataStoreReaderSync<F>>::get_contract_code_definition(self, contract_id).await
    }
    async fn get_latest_block_state(&self) -> anyhow::Result<PsyBlockState> {
        let latest_block_state = <Self as QMetaDataStoreReaderSync<F>>::get_latest_block_state(self).await?;

        // println!("got latest_block_state.checkpoint_id:
        // {}",latest_block_state.checkpoint_id);
        Ok(latest_block_state)
    }

    async fn get_block_state(&self, checkpoint_id: u64) -> anyhow::Result<PsyBlockState> {
        <Self as QMetaDataStoreReaderSync<F>>::get_block_state(self, checkpoint_id).await
    }

    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_registration_tree_root(self, checkpoint_id).await
    }
    async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_registration_tree_leaf_hash(self, checkpoint_id, leaf_index).await
    }
    async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_registration_tree_merkle_proof(self, checkpoint_id, leaf_index).await
    }
    async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_tree_root(self, checkpoint_id).await
    }
    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_user_sub_tree_merkle_proof(self, checkpoint_id, root_level, leaf_level, leaf_index).await
    }
    async fn get_user_top_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        UserTreeStore::<Self>::get_sub_tree_proof(
            self,
            GLOBAL_USER_TREE_HEIGHT as usize,
            0,
            &UserTreeStore::<Self>::new_node_key_fc(checkpoint_id, leaf_level, leaf_index),
        )
    }
    async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>> {
        UserTreeStore::<Self>::get_node(
            self,
            GLOBAL_USER_TREE_HEIGHT as usize,
            &UserTreeStore::<Self>::new_node_key_fc(checkpoint_id, cap_level, cap_index),
        )
    }
    async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>> {
        UserTreeStore::<Self>::get_node(
            self,
            GLOBAL_USER_TREE_HEIGHT as usize,
            &UserTreeStore::<Self>::new_node_key_fc(0xffffffffffu64, cap_level, cap_index),
        )
    }
    async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_contract_function_tree_root(self, checkpoint_id, contract_id).await
    }
    async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_contract_function_tree_leaf_hash(self, checkpoint_id, contract_id, function_id).await
    }
    async fn get_contract_function_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_contract_function_tree_merkle_proof(self, checkpoint_id, contract_id, function_id).await
    }
    async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_contract_tree_root(self, checkpoint_id).await
    }
    async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_contract_tree_leaf_hash(self, checkpoint_id, contract_id).await
    }
    async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_contract_tree_merkle_proof(self, checkpoint_id, contract_id).await
    }
    async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_deposit_tree_root(self, checkpoint_id).await
    }
    async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_deposit_tree_leaf_hash(self, checkpoint_id, deposit_id).await
    }
    async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_deposit_tree_merkle_proof(self, checkpoint_id, deposit_id).await
    }
    async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_withdrawal_tree_root(self, checkpoint_id).await
    }
    async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_withdrawal_tree_leaf_hash(self, checkpoint_id, withdrawal_id).await
    }
    async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QTreeDataStoreReaderSync<F>>::get_withdrawal_tree_merkle_proof(self, checkpoint_id, withdrawal_id).await
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
        let contract_tree_root = <Self as QTreeDataStoreReaderSync<F>>::get_contract_tree_root(self, checkpoint_id).await?;
        let deposit_tree_root = <Self as QTreeDataStoreReaderSync<F>>::get_deposit_tree_root(self, checkpoint_id).await?;
        let user_tree_root = <Self as QTreeDataStoreReaderSync<F>>::get_user_tree_root(self, checkpoint_id).await?;
        let withdrawal_tree_root = <Self as QTreeDataStoreReaderSync<F>>::get_withdrawal_tree_root(self, checkpoint_id).await?;
        let user_registration_tree_root = <Self as QTreeDataStoreReaderSync<F>>::get_user_registration_tree_root(self, checkpoint_id).await?;
        Ok(PsyCheckpointGlobalStateRoots {
            contract_tree_root,
            deposit_tree_root,
            user_tree_root,
            withdrawal_tree_root,
            user_registration_tree_root,
        })
    }
    async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> anyhow::Result<PsyCheckpointSyncInfoCompact<F>> {
        CheckpointSyncInfoTableStore::<Self>::get_checkpoint_sync_info_compact(self, checkpoint_id)
    }

    async fn get_first_user_id(&self, public_key: QHashOut<F>) -> anyhow::Result<u64> {
        Ok(
            UserPublicKeyTableStore::<Self>::get_first_user_for_public_key_hash_if_exists(self, public_key)?
                .ok_or(anyhow::anyhow!("User not found"))?
                .user_id,
        )
    }

    async fn get_realm_statuses(&self, realm_ids: &[u64]) -> anyhow::Result<Vec<BasicRealmStatus<F>>> {
        Ok(RealmStatusTableStore::<F, Self>::get_realm_statuses_by_id(self, realm_ids)?)
    }
}
