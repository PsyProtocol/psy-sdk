use kvq::traits::KVQBinaryStore;
use qed_core::data::qhashout::QHashOut;
use serde::{Deserialize, Serialize};

use crate::{config::store_config::{BaseContractStateTreeStore, QEDDeltaMerkleProof, QEDFelt, QEDHash, QEDMerkleProof, CONTRACT_STATE_TREE_ID, USER_CONTRACT_STATE_TREE_TABLE_TYPE}, models::kvq_merkle::{key::KVQMerkleNodeKey, model::{KVQMerkleTreeModelReaderCore, KVQMerkleTreeModelCore}}};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserContractStateTreeId {
    pub user_id: u64,
    pub contract_id: u32,
    pub height: u8,
}

impl UserContractStateTreeId {
    pub fn new(user_id: u64, contract_id: u32, height: u8) -> Self {
        Self {
            user_id,
            contract_id,
            height,
        }
    }
    pub fn get_leaf_key(&self, checkpoint_id: u64, index: u64) -> KVQMerkleNodeKey<USER_CONTRACT_STATE_TREE_TABLE_TYPE> {
        KVQMerkleNodeKey {
            tree_id: CONTRACT_STATE_TREE_ID,
            primary_id: self.user_id,
            secondary_id: self.contract_id,
            level: self.height,
            index,
            checkpoint_id,
        }
    }
    pub fn get_leaf_ucs<S: KVQBinaryStore>(&self, store: &S, checkpoint_id: u64, index: u64) -> anyhow::Result<QEDMerkleProof> {
        BaseContractStateTreeStore::<S>::get_leaf(store, &self.get_leaf_key(checkpoint_id, index))
    }
    pub fn get_leaf_value_ucs<S: KVQBinaryStore>(&self, store: &S, checkpoint_id: u64, index: u64) -> anyhow::Result<QHashOut<QEDFelt>> {
        BaseContractStateTreeStore::<S>::get_node(store, self.height.into(), &self.get_leaf_key(checkpoint_id, index))
    }
    pub fn set_leaf_ucs<S: KVQBinaryStore>(&self, store: &S, checkpoint_id: u64, index: u64, value: QEDHash) -> anyhow::Result<QEDDeltaMerkleProof> {
        BaseContractStateTreeStore::<S>::set_leaf(store, &self.get_leaf_key(checkpoint_id, index), value)
    }
    pub fn injest_merkle_proof_ucs<S: KVQBinaryStore>(&self, store: &S, checkpoint_id: u64, merkle_proof: &QEDMerkleProof) -> anyhow::Result<()> {
        BaseContractStateTreeStore::<S>::injest_merkle_proof(store, CONTRACT_STATE_TREE_ID, self.user_id, self.contract_id, checkpoint_id, merkle_proof)
    }
    pub fn get_root<S: KVQBinaryStore>(&self, store: &S, checkpoint_id: u64) -> anyhow::Result<QEDHash> {
        BaseContractStateTreeStore::<S>::get_node(store, self.height.into(), &KVQMerkleNodeKey{
            tree_id: CONTRACT_STATE_TREE_ID,
            primary_id: self.user_id,
            secondary_id: self.contract_id,
            level: 0,
            index: 0,
            checkpoint_id,
        })
    }

}
