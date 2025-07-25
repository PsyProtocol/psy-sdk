use kvq::traits::KVQSerializable;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::{hash::hash_types::RichField, plonk::config::AlgebraicHasher};
use qed_core::{
    data::qhashout::QHashOut,
    traits::to_qfelts::{QFeltSized, ToQFelts},
};
use qed_crypto::hash::traits::{hasher::FieldQHasher, qhashable::QFieldHashable};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::qdata::user::QEDUserLeaf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default, TS)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
#[ts(export, concrete(F = GoldilocksField))]
pub struct UserContractState<F: RichField> {
    pub checkpoint_tree_root: QHashOut<F>,
    pub user_leaf: QEDUserLeaf<F>,
}

impl<F: RichField> UserContractState<F> {
    pub fn new(
        checkpoint_tree_root: QHashOut<F>,
        contract_state_root: QHashOut<F>,
        user_leaf: QEDUserLeaf<F>,
    ) -> Self {
        Self {
            checkpoint_tree_root,
            user_leaf,
        }
    }
}
impl<F: RichField> KVQSerializable for UserContractState<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

impl<F: RichField> QFeltSized for UserContractState<F> {
    fn q_felt_size() -> usize {
        13 + 4
    }
}
impl<F: RichField> ToQFelts<F> for UserContractState<F> {
    fn to_qfelts(&self) -> Vec<F> {
        vec![
            self.checkpoint_tree_root.0.elements[0],
            self.checkpoint_tree_root.0.elements[1],
            self.checkpoint_tree_root.0.elements[2],
            self.checkpoint_tree_root.0.elements[3],
            self.user_leaf.public_key.0.elements[0],
            self.user_leaf.public_key.0.elements[1],
            self.user_leaf.public_key.0.elements[2],
            self.user_leaf.public_key.0.elements[3],
            self.user_leaf.user_state_tree_root.0.elements[0],
            self.user_leaf.user_state_tree_root.0.elements[1],
            self.user_leaf.user_state_tree_root.0.elements[2],
            self.user_leaf.user_state_tree_root.0.elements[3],
            self.user_leaf.balance,
            self.user_leaf.nonce,
            self.user_leaf.last_checkpoint_id,
            self.user_leaf.event_index,
            self.user_leaf.user_id,
        ]
    }

    fn from_qfelts(felts: &[F]) -> Self {
        if felts.len() != Self::q_felt_size() {
            panic!("Invalid number of elements for UserContractState");
        }

        UserContractState {
            checkpoint_tree_root: QHashOut::from_qfelts(&felts[0..4]),
            user_leaf: QEDUserLeaf::from_qfelts(&felts[8..]),
        }
    }
}

impl<F: RichField> QFieldHashable<F> for UserContractState<F> {
    fn qfhash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {
        H::q_hash_many(&[
            self.checkpoint_tree_root.0.elements[0],
            self.checkpoint_tree_root.0.elements[1],
            self.checkpoint_tree_root.0.elements[2],
            self.checkpoint_tree_root.0.elements[3],
            self.user_leaf.public_key.0.elements[0],
            self.user_leaf.public_key.0.elements[1],
            self.user_leaf.public_key.0.elements[2],
            self.user_leaf.public_key.0.elements[3],
            self.user_leaf.user_state_tree_root.0.elements[0],
            self.user_leaf.user_state_tree_root.0.elements[1],
            self.user_leaf.user_state_tree_root.0.elements[2],
            self.user_leaf.user_state_tree_root.0.elements[3],
            self.user_leaf.balance,
            self.user_leaf.nonce,
            self.user_leaf.last_checkpoint_id,
            self.user_leaf.event_index,
            self.user_leaf.user_id,
        ])
    }
}

impl<F: RichField> UserContractState<F> {
    pub fn alghash<H: AlgebraicHasher<F>>(&self) -> QHashOut<F> {
        QHashOut(H::hash_no_pad(&[
            self.checkpoint_tree_root.0.elements[0],
            self.checkpoint_tree_root.0.elements[1],
            self.checkpoint_tree_root.0.elements[2],
            self.checkpoint_tree_root.0.elements[3],
            self.user_leaf.public_key.0.elements[0],
            self.user_leaf.public_key.0.elements[1],
            self.user_leaf.public_key.0.elements[2],
            self.user_leaf.public_key.0.elements[3],
            self.user_leaf.user_state_tree_root.0.elements[0],
            self.user_leaf.user_state_tree_root.0.elements[1],
            self.user_leaf.user_state_tree_root.0.elements[2],
            self.user_leaf.user_state_tree_root.0.elements[3],
            self.user_leaf.balance,
            self.user_leaf.nonce,
            self.user_leaf.last_checkpoint_id,
            self.user_leaf.event_index,
            self.user_leaf.user_id,
        ]))
    }
}
