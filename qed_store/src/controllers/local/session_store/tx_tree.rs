use std::marker::PhantomData;

use kvq::traits::{KVQBinaryStore, KVQBinaryStoreImmutable, KVQSerializable};
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::{merkle::core::{DeltaMerkleProofCore, MerkleProofCore}, traits::qhashable::QFieldHashable};

use crate::{config::store_config::{QEDFelt, QEDHasher}, models::kvq_merkle::model::{KVQFixedConfigMerkleTreeModelCore, KVQFixedConfigMerkleTreeModelCoreImmutable, KVQFixedConfigMerkleTreeModelReaderCore}};

use super::config::{LPSDeferredTransactionTreeStore, LocalProvingSessionTreeStore};

type GF = QEDFelt;
type QHasher = QEDHasher;

#[derive(Clone, Debug)]
pub struct TransactionDebtItem<TX: QFieldHashable<F>, F: RichField> {
    pub call_data: TX,
    pub tree_index: u64,
    pub hash: QHashOut<F>,
    pub insertion_proof: DeltaMerkleProofCore<QHashOut<F>>,
}
#[derive(Clone, Debug)]
pub struct TransactionDebtTreeRef<TX: QFieldHashable<F>, F: RichField, const HEIGHT: u8, const TREE_ID: u8> {
    _tx: PhantomData<TX>,
    _f: PhantomData<F>,
    next_index: usize,
    checkpoint_id: u64,
    remaining_debt: Vec<TransactionDebtItem<TX, F>>,

}


impl<TX: KVQSerializable + QFieldHashable<F>, F: RichField, const HEIGHT: u8, const TREE_ID: u8> TransactionDebtTreeRef<TX, F, HEIGHT, TREE_ID> {
    pub fn new(checkpoint_id: u64) -> Self {
        Self {
            _tx: PhantomData::default(),
            _f: PhantomData::default(),
            next_index: 0,
            checkpoint_id,
            remaining_debt: vec![]
        }
    }
    pub fn get_remaining_proof_debt(&self) -> usize {
        self.remaining_debt.len()
    }
    pub fn get_next_index(&self) -> usize {
        self.next_index
    }
}


impl<TX: KVQSerializable + QFieldHashable<GF>, const HEIGHT: u8, const TREE_ID: u8> TransactionDebtTreeRef<TX, GF, HEIGHT, TREE_ID> {
    pub fn get_latest_tx_debt_leaf<S: KVQBinaryStore>(&self, store: &S) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
        let index = self.get_next_index() as u64;
        if index > 0 {
            self.get_tx_debt_leaf(store, index-1)
        }else{
            self.get_tx_debt_leaf(store, index)
        }
        
    }
    pub fn get_tx_debt_leaf<S: KVQBinaryStore>(&self, store: &S, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
            LocalProvingSessionTreeStore::<S, TREE_ID, HEIGHT>::get_leaf_fc(store, self.checkpoint_id, leaf_index)
    }
    pub fn add_tx_debt<S: KVQBinaryStore>(&mut self, store: &mut S, call_data: TX) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        let new_index = (self.next_index as u64);
        self.next_index += 1;
        let hash = call_data.qfhash::<QHasher>();

        let insertion_proof = LocalProvingSessionTreeStore::<S, TREE_ID, HEIGHT>::set_leaf_fc(store, self.checkpoint_id, new_index, hash)?;
        let debt_item = TransactionDebtItem {
            call_data,
            tree_index: new_index,
            hash,
            insertion_proof: insertion_proof.clone(),
        };
        self.remaining_debt.push(debt_item);
        Ok(insertion_proof)        
    } 
    
    pub fn add_tx_debt_imm<S: KVQBinaryStoreImmutable>(&mut self, store: &S, call_data: TX) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        let new_index = (self.next_index as u64);
        self.next_index += 1;
        let hash = call_data.qfhash::<QHasher>();

        let insertion_proof = LocalProvingSessionTreeStore::<S, TREE_ID, HEIGHT>::set_leaf_fc_imm(store, self.checkpoint_id, new_index, hash)?;
        let debt_item = TransactionDebtItem {
            call_data,
            tree_index: new_index,
            hash,
            insertion_proof: insertion_proof.clone(),
        };
        self.remaining_debt.push(debt_item);
        Ok(insertion_proof)        
    }

}

/*

    pub fn add_tx_debt<S: KVQBinaryStore>(&mut self, store: &mut S, call_data: TX) -> QHashOut<F> {
        let new_index = F::from_noncanonical_u64(self.next_index as u64);
        self.next_index += 1;
        let hash = call_data.qfhash::<QHasher>();

        LPSDeferredTransactionTreeStore::<S>::set_leaf_fc(store, self.checkpoint_id, new_index, hash)
    } */