use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::core::{ DeltaMerkleProofCore, MerkleProofCore};

use super::qmetadata::QMetaDataStoreReaderSync;


pub trait ActiveCheckpointReaderSync<F: RichField> {
    fn get_active_checkpoint(&self) -> anyhow::Result<u64>;
    fn get_active_checkpoint_f(&self) -> anyhow::Result<F>;
    fn get_active_writing_checkpoint(&self) -> anyhow::Result<u64>;
    fn get_active_writing_checkpoint_f(&self) -> anyhow::Result<F>;
}
pub trait ActiveCheckpointWriterSync<F: RichField> {
    fn set_active_checkpoint(&self, checkpoint_id: u64) -> anyhow::Result<u64>;
    fn set_active_checkpoint_f(&self, checkpoint_id: F) -> anyhow::Result<F>;
    fn set_active_writing_checkpoint(&self, checkpoint_id: u64) -> anyhow::Result<u64>;
    fn set_active_writing_checkpoint_f(&self, checkpoint_id: F) -> anyhow::Result<F>;
    
    fn set_active_checkpoint_mut(&mut self, checkpoint_id: u64) -> anyhow::Result<u64>;
    fn set_active_checkpoint_f_mut(&mut self, checkpoint_id: F) -> anyhow::Result<F>;
    fn set_active_writing_checkpoint_mut(&mut self, checkpoint_id: u64) -> anyhow::Result<u64>;
    fn set_active_writing_checkpoint_f_mut(&mut self, checkpoint_id: F) -> anyhow::Result<F>;
}
pub trait QTreeDataStoreReaderSync<F: RichField> {
    fn get_user_contract_state_tree_root(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    fn get_user_contract_state_tree_root_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_user_contract_state_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32, height: u8, leaf_id: u64) -> anyhow::Result<QHashOut<F>>;
    fn get_user_contract_state_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F, height: u8, leaf_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_user_contract_state_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64, contract_id: u32, height: u8, leaf_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    fn get_user_contract_state_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F, contract_id: F, height: u8, leaf_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;


    fn get_user_contract_tree_root(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QHashOut<F>>;
    fn get_user_contract_tree_root_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_user_contract_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    fn get_user_contract_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_user_contract_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    fn get_user_contract_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    

    fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_user_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QHashOut<F>>;
    fn get_user_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_user_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    fn get_user_tree_merkle_proof_f(&self, checkpoint_id: F, user_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;


    fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>>;
    fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_contract_function_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    fn get_contract_function_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    


    fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    
    
    
    fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>>;
    fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;


    fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>>;
    fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    
    fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>>;
    fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u32) -> anyhow::Result<QHashOut<F>>;
    fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
}

pub trait QEDComboDataStoreReaderSync<F: RichField>: QMetaDataStoreReaderSync<F> + QTreeDataStoreReaderSync<F> {}

pub trait QTreeDataStoreWriterSync<F: RichField> {
    fn set_user_state_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32, height: u8, leaf_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_user_state_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F, height: u8, leaf_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;

    fn set_user_contract_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_user_contract_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, contract_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;

    fn set_user_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_user_tree_leaf_hash_f(&self, checkpoint_id: F, user_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;

    fn set_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    
    fn set_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    

    fn set_contract_function_whitelist(&self, checkpoint_id: u64, contract_id: u64, leaves: &[QHashOut<F>]) -> anyhow::Result<QHashOut<F>>;
    fn set_contract_function_whitelist_f(&self, checkpoint_id: F, contract_id: F, leaves: &[QHashOut<F>]) -> anyhow::Result<QHashOut<F>>;
    

    fn set_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    
    fn set_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
    fn set_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_hash: QHashOut<F>) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<F>>>;
}
