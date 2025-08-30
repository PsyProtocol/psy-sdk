use async_trait::async_trait;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;

#[derive(Debug, Clone)]
pub struct InitializeParams<F: RichField> {
    pub gutas_root: QHashOut<F>,
    pub deploy_contracts_root: QHashOut<F>,
    pub register_users_root: QHashOut<F>,
    pub next_contract_id: u32,
    pub next_user_id: u64,
}

impl<F: RichField> Default for InitializeParams<F> {
    fn default() -> Self {
        Self {
            gutas_root: QHashOut::ZERO,
            deploy_contracts_root: QHashOut::ZERO,
            register_users_root: QHashOut::ZERO,
            next_contract_id: 0,
            next_user_id: 0,
        }
    }
}
use qed_crypto::hash::merkle::{core::{DeltaMerkleProofCore, MerkleProofCore}, spiderman::SpidermanUpdateProof, utils::{common::QMerkleNode, sub_tree_nca::{NCAProofsWithTopLine, UpdateNCAProofsWithDependencies}}};
use qed_data::{config::store_config::UserPublicKeyTableStore, models::checkpoint::user_public_keys::QEDUserPublicKeyHelperModelCore, qdata::{checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState}, contract::{ContractCodeDefinition, QEDContractLeaf}, user_public_key::QEDUserPublicKeyRecord}, qsync::coordinator::QEDCheckpointSyncInfoCompact};

pub mod reader_async;
pub mod writer_imm;

#[async_trait]
pub trait QEDCoordinatorStoreReaderAsync<F: RichField> {
    async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<F>>;
    async fn get_contract_leaf_data_f(&self, contract_id: F) -> anyhow::Result<QEDContractLeaf<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_contract_leaf_data(self, contract_id.to_canonical_u64()).await
    }

    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_checkpoint_leaf_data(self, checkpoint_id.to_canonical_u64()).await
    }

    async fn get_contract_code_definition(&self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition>;
    async fn get_contract_code_definition_f(&self, contract_id: F) -> anyhow::Result<ContractCodeDefinition> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_contract_code_definition(self, contract_id.to_canonical_u64()).await
    }
    async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState>;

    async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState>;
    async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_l2_block_state(self, checkpoint_id.to_canonical_u64()).await
    }



    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_user_registration_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_user_registration_tree_leaf_hash(self, checkpoint_id.to_canonical_u64(), leaf_index.to_canonical_u64()).await
    }
    async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_user_registration_tree_merkle_proof(self, checkpoint_id.to_canonical_u64(), leaf_index.to_canonical_u64()).await
    }


    async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_user_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_user_sub_tree_merkle_proof(&self, checkpoint_id: u64, root_level: u8, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_top_tree_merkle_proof(&self, checkpoint_id: u64, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;


    async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_contract_function_tree_root(self, checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32).await
    }
    async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_contract_function_tree_leaf_hash(self, checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32, function_id.to_canonical_u64() as u32).await
    }
    async fn get_contract_function_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_contract_function_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_contract_function_tree_merkle_proof(self, checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32, function_id.to_canonical_u64() as u32).await
    }



    async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_contract_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_contract_tree_leaf_hash(self, checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32).await
    }
    async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_contract_tree_merkle_proof(self, checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64() as u32).await
    }



    async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_deposit_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_deposit_tree_leaf_hash(self, checkpoint_id.to_canonical_u64(), deposit_id.to_canonical_u64() as u32).await
    }
    async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_deposit_tree_merkle_proof(self, checkpoint_id.to_canonical_u64(), deposit_id.to_canonical_u64() as u32).await
    }


    async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_withdrawal_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>>;
    async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_withdrawal_tree_leaf_hash(self, checkpoint_id.to_canonical_u64(), withdrawal_id.to_canonical_u64() as u32).await
    }
    async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_withdrawal_tree_merkle_proof(self, checkpoint_id.to_canonical_u64(), withdrawal_id.to_canonical_u64() as u32).await
    }

    async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_checkpoint_tree_root(self, checkpoint_id.to_canonical_u64()).await
    }
    async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_checkpoint_tree_leaf_hash(self, checkpoint_id.to_canonical_u64(), leaf_checkpoint_id.to_canonical_u64()).await
    }
    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        <Self as QEDCoordinatorStoreReaderAsync<F>>::get_checkpoint_tree_merkle_proof(self, checkpoint_id.to_canonical_u64(), leaf_checkpoint_id.to_canonical_u64()).await
    }

    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointGlobalStateRoots<F>>;
    async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointSyncInfoCompact<F>>;

    async fn get_first_user_id(&self, public_key: QHashOut<F>) -> anyhow::Result<u64>;
}


#[async_trait]
pub trait QEDCoordinatorStoreWriterAsyncImm<F: RichField> {
    async fn batch_append_user_registration_tree_imm(&self, checkpoint_id: u64, start_leaf_index: u64, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>>;
    async fn batch_append_user_registration_tree_f_imm(&self, checkpoint_id: F, start_leaf_index: F, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>> {
        <Self as QEDCoordinatorStoreWriterAsyncImm<F>>::batch_append_user_registration_tree_imm(self, checkpoint_id.to_canonical_u64(), start_leaf_index.to_canonical_u64(), sub_tree_height, leaf_hashes).await
    }

    async fn injest_user_tree_nodes_imm(&self, checkpoint_id: u64, root_level: u8, nodes: &[QMerkleNode<F>]) -> anyhow::Result<UpdateNCAProofsWithDependencies<QHashOut<F>>>;


    async fn set_deposit_tree_leaf_hash_imm(&self, checkpoint_id: u64, deposit_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    async fn set_deposit_tree_leaf_hash_f_imm(&self, checkpoint_id: F, deposit_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QEDCoordinatorStoreWriterAsyncImm<F>>::set_deposit_tree_leaf_hash_imm(self, checkpoint_id.to_canonical_u64(), deposit_id.to_canonical_u64(), leaf_hash).await
    }

    async fn set_withdrawal_tree_leaf_hash_imm(&self, checkpoint_id: u64, withdrawal_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    async fn set_withdrawal_tree_leaf_hash_f_imm(&self, checkpoint_id: F, withdrawal_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QEDCoordinatorStoreWriterAsyncImm<F>>::set_withdrawal_tree_leaf_hash_imm(self, checkpoint_id.to_canonical_u64(), withdrawal_id.to_canonical_u64(), leaf_hash).await
    }


    async fn set_contract_function_whitelist_imm(&self, checkpoint_id: u64, contract_id: u64, leaves: &[QHashOut<F>]) -> anyhow::Result<QHashOut<F>>;
    async fn set_contract_function_whitelist_f_imm(&self, checkpoint_id: F, contract_id: F, leaves: &[QHashOut<F>]) -> anyhow::Result<QHashOut<F>> {
        <Self as QEDCoordinatorStoreWriterAsyncImm<F>>::set_contract_function_whitelist_imm(self, checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64(), leaves).await
    }

    async fn batch_append_contract_tree_imm(&self, checkpoint_id: u64, start_leaf_index: u64, sub_tree_height: u8, leaf_hashes: &[QHashOut<F>]) -> anyhow::Result<Vec<SpidermanUpdateProof<QHashOut<F>>>>;

    async fn set_contract_tree_leaf_hash_imm(&self, checkpoint_id: u64, contract_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    async fn set_contract_tree_leaf_hash_f_imm(&self, checkpoint_id: F, contract_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QEDCoordinatorStoreWriterAsyncImm<F>>::set_contract_tree_leaf_hash_imm(self, checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64(), leaf_hash).await
    }

    async fn set_checkpoint_tree_leaf_hash_imm(&self, checkpoint_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    async fn set_checkpoint_tree_leaf_hash_f_imm(&self, checkpoint_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>> {
        <Self as QEDCoordinatorStoreWriterAsyncImm<F>>::set_checkpoint_tree_leaf_hash_imm(self, checkpoint_id.to_canonical_u64(), leaf_hash).await
    }



    async fn set_contract_leaf_data_imm(&self, checkpoint_id: u64, contract_id: u64, leaf_data: &QEDContractLeaf<F>) -> anyhow::Result<()>;
    async fn set_contract_leaf_data_f_imm(&self, checkpoint_id: F, contract_id: F, leaf_data: &QEDContractLeaf<F>) -> anyhow::Result<()> {
        <Self as QEDCoordinatorStoreWriterAsyncImm<F>>::set_contract_leaf_data_imm(self, checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64(), leaf_data).await
    }

    async fn set_checkpoint_leaf_data_imm(&self, checkpoint_id: u64, leaf_data: &QEDCheckpointLeaf<F>) -> anyhow::Result<()>;
    async fn set_checkpoint_leaf_data_f_imm(&self, checkpoint_id: F, leaf_data: &QEDCheckpointLeaf<F>) -> anyhow::Result<()> {
        <Self as QEDCoordinatorStoreWriterAsyncImm<F>>::set_checkpoint_leaf_data_imm(self, checkpoint_id.to_canonical_u64(), leaf_data).await
    }

    async fn set_contract_code_definition_imm(&self, checkpoint_id: u64, contract_id: u64, definition: &ContractCodeDefinition) -> anyhow::Result<()>;
    async fn set_contract_code_definition_f_imm(&self, checkpoint_id: F, contract_id: F, definition: &ContractCodeDefinition) -> anyhow::Result<()> {
        <Self as QEDCoordinatorStoreWriterAsyncImm<F>>::set_contract_code_definition_imm(self, checkpoint_id.to_canonical_u64(), contract_id.to_canonical_u64(), definition).await
    }

    async fn set_l2_block_state_imm(&self, block_state: &QEDL2BlockState) -> anyhow::Result<()>;
    async fn set_checkpoint_sync_info_imm(&self, sync_info: QEDCheckpointSyncInfoCompact<F>) -> anyhow::Result<()>;
    async fn initialize_store(&self, params: InitializeParams<F>) -> anyhow::Result<u64>;

    async fn set_user_public_key_records(&self, records: &[QEDUserPublicKeyRecord<F>]) -> anyhow::Result<()>;

}
