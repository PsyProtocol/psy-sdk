use async_trait::async_trait;
use kvq::traits::KVQPair;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::{core::{DeltaMerkleProofCore, MerkleProofCore}, utils::{common::QMerkleNode, sub_tree_nca::{NCAProofsWithTopLine, UpdateNCAProofsWithDependencies}}};
use qed_data::{qdata::{checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState}, contract::{ContractCodeDefinition, QEDContractLeaf}, user::QEDUserLeaf}, qstore::uct_merkle_nodes::CSTUserUpdate, qsync::coordinator::QEDCheckpointSyncInfo};

use qed_data::models::kvq_merkle::key::KVQMerkleNodeKey;

pub mod reader_async;
pub mod writer_imm;

#[async_trait]
pub trait QEDRealmStoreReaderAsync<F: RichField> {
    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_checkpoint_leaf_data(self, checkpoint_id.to_canonical_u64()).await
    }

    async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState>;

    async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState>;
    async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_l2_block_state(self, checkpoint_id.to_canonical_u64()).await
    }


    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;


    async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_checkpoint_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_checkpoint_tree_leaf_hash(self, checkpoint_id.to_canonical_u64(), leaf_checkpoint_id.to_canonical_u64()).await
    }
    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_checkpoint_tree_merkle_proof(self, checkpoint_id.to_canonical_u64(), leaf_checkpoint_id.to_canonical_u64()).await
    }

    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointGlobalStateRoots<F>>;



    async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QEDUserLeaf<F>>;
    async fn get_user_leaf_data_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<QEDUserLeaf<F>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_user_leaf_data(self, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }


    async fn get_user_contract_state_tree_root(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_state_tree_root_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_user_contract_state_tree_root(self, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32).await
    }
    async fn get_user_contract_state_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32, height: u8, leaf_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_state_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F, height: u8, leaf_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_user_contract_state_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            height,
            leaf_id.to_canonical_u64()
        ).await
    }
    async fn get_user_contract_state_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64, contract_id: u32, height: u8, leaf_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_contract_state_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F, contract_id: F, height: u8, leaf_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_user_contract_state_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32,
            height,
            leaf_id.to_canonical_u64()
        ).await
    }


    async fn get_user_contract_tree_root(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_tree_root_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_user_contract_tree_root(self, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }
    async fn get_user_contract_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_contract_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_user_contract_tree_leaf_hash(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32
        ).await
    }
    async fn get_user_contract_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_contract_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_user_contract_tree_merkle_proof(
            self,
            checkpoint_id.to_canonical_u64(),
            user_id.to_canonical_u64(),
            contract_id.to_canonical_u64() as u32
        ).await
    }



    async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_user_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_user_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_user_tree_leaf_hash(self, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }
    async fn get_user_bottom_tree_merkle_proof(&self, root_level: u8,checkpoint_id: u64, user_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_bottom_tree_merkle_proof_f(&self, root_level: u8, checkpoint_id: F, user_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_user_bottom_tree_merkle_proof(self, root_level, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }
    async fn get_user_sub_tree_merkle_proof(&self, checkpoint_id: u64, root_level: u8, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;

    async fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> ;

    async fn get_user_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QEDRealmStoreReaderAsync<F>>::get_user_tree_merkle_proof(self, checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64()).await
    }
}

#[async_trait]
pub trait QEDRealmStoreWriterAsyncImm<F: RichField> {
    async fn injest_user_tree_nodes_imm(&self, checkpoint_id: u64, root_level: u8, nodes: &[QMerkleNode<F>]) -> anyhow::Result<UpdateNCAProofsWithDependencies<QHashOut<F>>>;
    async fn injest_user_leaves_imm(&self, checkpoint_id: u64, root_level: u8, leaves: &[QEDUserLeaf<F>]) -> anyhow::Result<Vec<DeltaMerkleProofCore<QHashOut<F>>>>;
    async fn injest_user_leaves_batch_imm(&self, checkpoint_id: u64, leaves: &[QEDUserLeaf<F>]) -> anyhow::Result<()>;
    async fn injest_checked_cst_nodes_imm(&self, user_updates: &[CSTUserUpdate<QHashOut<F>>]) -> anyhow::Result<()>;
    async fn injest_checkpoint_sync_data_imm(&self, sync_info: QEDCheckpointSyncInfo<F>) -> anyhow::Result<()>;


    async fn set_contract_leaf_data_imm(&self, checkpoint_id: u64, contract_id: u64, leaf_data: &QEDContractLeaf<F>) -> anyhow::Result<()>;
    async fn set_contract_leaf_data_f_imm(&self, checkpoint_id: F, contract_id: F, leaf_data: &QEDContractLeaf<F>) -> anyhow::Result<()> {
        <Self as QEDRealmStoreWriterAsyncImm<F>>::set_contract_leaf_data_imm(self, checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64(), leaf_data).await
    }

    async fn set_contract_code_definition_imm(&self, checkpoint_id: u64, contract_id: u64, definition: &ContractCodeDefinition) -> anyhow::Result<()>;
    async fn set_contract_code_definition_f_imm(&self, checkpoint_id: F, contract_id: F, definition: &ContractCodeDefinition) -> anyhow::Result<()> {
        <Self as QEDRealmStoreWriterAsyncImm<F>>::set_contract_code_definition_imm(self, checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64(), definition).await
    }

    async fn commit_block_imm(&self, checkpoint_id: u64) -> anyhow::Result<()>;
}
