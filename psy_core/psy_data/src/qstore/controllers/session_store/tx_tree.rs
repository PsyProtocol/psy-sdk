use std::marker::PhantomData;

use kvq::{
    adapters::standard::KVQStandardAdapter,
    traits::{KVQSerializable, KVQStoreAdapter},
};
use plonky2::hash::hash_types::RichField;
use psy_common::data::qhashout::QHashOut;
use psy_crypto::hash::{
    merkle::core::{DeltaMerkleProofCore, MerkleProofCore},
    traits::qhashable::QFieldHashable,
};
use serde::{Deserialize, Serialize};

use super::config::{LocalProvingSessionTreeStore, LOCAL_PROVING_SESSION_TREE_TABLE_TYPE};
use crate::{
    config::store_config::{PsyFelt, PsyHash, PsyHasher},
    dpn::proving_session::DPNTransactionDebtItem,
    models::kvq_merkle::{
        key::KVQMerkleNodeKey,
        model::{KVQFixedConfigMerkleTreeModelCore, KVQFixedConfigMerkleTreeModelReaderCore},
    },
};

type GF = PsyFelt;
type QHasher = PsyHasher;
#[derive(Serialize)]
#[serde(bound = "for<'de2> TX: Deserialize<'de2>")]
pub struct TransactionDebtTreeRef<
    S,
    TX: QFieldHashable<F> + Serialize,
    F: RichField,
    const HEIGHT: u8,
    const TREE_ID: u8,
    IDKVA = KVQStandardAdapter<S, KVQMerkleNodeKey<{ LOCAL_PROVING_SESSION_TREE_TABLE_TYPE }>, PsyHash>,
> {
    _tx: PhantomData<TX>,
    _f: PhantomData<F>,
    #[serde(skip)]
    _adapter: PhantomData<(S, IDKVA)>,
    next_index: u64,
    checkpoint_id: u64,
    remaining_debt: Vec<DPNTransactionDebtItem<TX, F>>,
}

impl<
        S,
        TX: KVQSerializable + QFieldHashable<F> + Serialize,
        F: RichField,
        const HEIGHT: u8,
        const TREE_ID: u8,
        IDKVA: KVQStoreAdapter<S, KVQMerkleNodeKey<{ LOCAL_PROVING_SESSION_TREE_TABLE_TYPE }>, PsyHash>,
    > TransactionDebtTreeRef<S, TX, F, HEIGHT, TREE_ID, IDKVA>
{
    pub fn new(checkpoint_id: u64) -> Self {
        Self {
            _tx: PhantomData::default(),
            _f: PhantomData::default(),
            _adapter: PhantomData::default(),
            next_index: 0,
            checkpoint_id,
            remaining_debt: vec![],
        }
    }
    pub fn has_remaining_proof_debt(&self) -> bool {
        self.remaining_debt.len() != 0
    }
    pub fn get_remaining_proof_debt(&self) -> usize {
        self.remaining_debt.len()
    }
    pub fn get_next_index(&self) -> u64 {
        self.next_index
    }
    pub fn get_latest_index(&self) -> u64 {
        match self.get_latest_proof_debt_item() {
            Some(item) => item.tree_index,
            None => 0,
        }
    }
    pub fn get_proof_debt_array(&self) -> &Vec<DPNTransactionDebtItem<TX, F>> {
        &self.remaining_debt
    }
    pub fn get_proof_debt_item_by_tree_index_usize(&self, tree_index: usize) -> Option<&DPNTransactionDebtItem<TX, F>> {
        self.get_proof_debt_item_by_tree_index(tree_index as u64)
    }
    pub fn get_proof_debt_item_by_tree_index(&self, tree_index: u64) -> Option<&DPNTransactionDebtItem<TX, F>> {
        self.remaining_debt.iter().find(|x| x.tree_index == tree_index)
    }
    pub fn get_latest_proof_debt_item(&self) -> Option<&DPNTransactionDebtItem<TX, F>> {
        self.remaining_debt.last()
    }

    // private mutable helpers
    fn remove_proof_debt_item_by_tree_index_u64(&mut self, tree_index: u64) -> Option<DPNTransactionDebtItem<TX, F>> {
        if self.remaining_debt.len() == 0 {
            None
        } else {
            match self.remaining_debt.iter().position(|x| x.tree_index == tree_index) {
                Some(array_index) => Some(self.remaining_debt.remove(array_index)),
                None => None,
            }
        }
    }
}

impl<
        S,
        TX: KVQSerializable + QFieldHashable<GF> + Serialize,
        const HEIGHT: u8,
        const TREE_ID: u8,
        IDKVA: KVQStoreAdapter<S, KVQMerkleNodeKey<{ LOCAL_PROVING_SESSION_TREE_TABLE_TYPE }>, PsyHash>,
    > TransactionDebtTreeRef<S, TX, GF, HEIGHT, TREE_ID, IDKVA>
{
    pub fn get_latest_tx_debt_leaf(&self, store: &S) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
        self.get_tx_debt_leaf(store, self.get_latest_index())
    }
    pub fn get_tx_debt_leaf(&self, store: &S, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<GF>>> {
        LocalProvingSessionTreeStore::<S, TREE_ID, HEIGHT, IDKVA>::get_leaf_fc(store, self.checkpoint_id, leaf_index)
    }
    pub fn add_tx_debt(&mut self, store: &S, call_data: TX) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        let new_index = self.next_index as u64;
        self.next_index += 1;
        let hash = call_data.qfhash::<QHasher>();

        let insertion_proof = LocalProvingSessionTreeStore::<S, TREE_ID, HEIGHT, IDKVA>::set_leaf_fc(store, self.checkpoint_id, new_index, hash)?;
        let debt_item = DPNTransactionDebtItem {
            call_data,
            tree_index: new_index,
            hash,
            insertion_proof: insertion_proof.clone(),
        };
        self.remaining_debt.push(debt_item);
        Ok(insertion_proof)
    }

    pub fn add_tx_debt_imm(&mut self, store: &S, call_data: TX) -> anyhow::Result<DeltaMerkleProofCore<QHashOut<GF>>> {
        let new_index = self.next_index as u64;
        self.next_index += 1;
        let hash = call_data.qfhash::<QHasher>();

        let insertion_proof = LocalProvingSessionTreeStore::<S, TREE_ID, HEIGHT, IDKVA>::set_leaf_fc(store, self.checkpoint_id, new_index, hash)?;
        let debt_item = DPNTransactionDebtItem {
            call_data,
            tree_index: new_index,
            hash,
            insertion_proof: insertion_proof.clone(),
        };
        self.remaining_debt.push(debt_item);
        Ok(insertion_proof)
    }
    pub fn repay_tx_debt(
        &mut self,
        store: &S,
        tree_leaf_index: u64,
    ) -> anyhow::Result<(DPNTransactionDebtItem<TX, GF>, DeltaMerkleProofCore<QHashOut<GF>>)> {
        let removed = self.remove_proof_debt_item_by_tree_index_u64(tree_leaf_index);
        match removed {
            Some(item) => {
                let removal_proof = LocalProvingSessionTreeStore::<S, TREE_ID, HEIGHT, IDKVA>::set_leaf_fc(
                    store,
                    self.checkpoint_id,
                    tree_leaf_index,
                    QHashOut::ZERO,
                )?;

                // simple tree compaction
                if self.has_remaining_proof_debt() {
                    if self.next_index == tree_leaf_index + 1 {
                        self.next_index = tree_leaf_index;
                    }
                } else {
                    self.next_index = 0;
                }

                Ok((item, removal_proof))
            }
            None => anyhow::bail!("transaction debt not found at tree index {}", tree_leaf_index),
        }
    }
    pub fn repay_tx_debt_imm(
        &mut self,
        store: &S,
        tree_leaf_index: u64,
    ) -> anyhow::Result<(DPNTransactionDebtItem<TX, GF>, DeltaMerkleProofCore<QHashOut<GF>>)> {
        let removed = self.remove_proof_debt_item_by_tree_index_u64(tree_leaf_index);
        match removed {
            Some(item) => {
                let removal_proof = LocalProvingSessionTreeStore::<S, TREE_ID, HEIGHT, IDKVA>::set_leaf_fc(
                    store,
                    self.checkpoint_id,
                    tree_leaf_index,
                    QHashOut::ZERO,
                )?;

                // simple tree compaction
                if self.has_remaining_proof_debt() {
                    if self.next_index == tree_leaf_index + 1 {
                        self.next_index = tree_leaf_index;
                    }
                } else {
                    self.next_index = 0;
                }

                Ok((item, removal_proof))
            }
            None => anyhow::bail!("transaction debt not found at tree index {}", tree_leaf_index),
        }
    }
}
