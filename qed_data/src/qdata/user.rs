use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::{data::qhashout::QHashOut, traits::to_qfelts::{QFeltSized, ToQFelts}};
use qed_crypto::hash::traits::{hasher::FieldHasher, qhashable::QFieldHashable};
use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDUserLeaf<F: RichField> {
    pub public_key: QHashOut<F>,
    pub user_state_tree_root: QHashOut<F>,
    pub balance: F,
    pub nonce: F,
    pub last_checkpoint_id: F,
    pub event_index: F,
    pub user_id: F,
}

impl<F: RichField> KVQSerializable for QEDUserLeaf<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

impl<F: RichField> QFeltSized for QEDUserLeaf<F> {
    fn q_felt_size() -> usize {
        13
    }
}
impl<F: RichField> ToQFelts<F> for QEDUserLeaf<F> {
    fn to_qfelts(&self) -> Vec<F> {
        vec![
            self.public_key.0.elements[0],
            self.public_key.0.elements[1],
            self.public_key.0.elements[2],
            self.public_key.0.elements[3],
            self.user_state_tree_root.0.elements[0],
            self.user_state_tree_root.0.elements[1],
            self.user_state_tree_root.0.elements[2],
            self.user_state_tree_root.0.elements[3],
            self.balance,
            self.nonce,
            self.last_checkpoint_id,
            self.event_index,
            self.user_id,
        ]
    }

    fn from_qfelts(felts: &[F]) -> Self {
        if felts.len() != 13 {
            panic!("Invalid number of elements for QEDUserLeaf");
        }
        let public_key = QHashOut::from_qfelts(&felts[0..4]);
        let user_state_tree_root = QHashOut::from_qfelts(&felts[4..8]);
        let balance = felts[8];
        let nonce = felts[9];
        let last_checkpoint_id = felts[10];
        let event_index = felts[11];
        let user_id = felts[12];
        QEDUserLeaf {
            public_key,
            user_state_tree_root,
            balance,
            nonce,
            last_checkpoint_id,
            event_index,
            user_id,
        }
    }
}

impl<F: RichField> QFieldHashable<F> for QEDUserLeaf<F> {
    fn qfhash<H: FieldHasher<QHashOut<F>, F>>(&self) -> QHashOut<F> {
        H::hash_many(&[
            self.public_key.0.elements[0],
            self.public_key.0.elements[1],
            self.public_key.0.elements[2],
            self.public_key.0.elements[3],
            self.user_state_tree_root.0.elements[0],
            self.user_state_tree_root.0.elements[1],
            self.user_state_tree_root.0.elements[2],
            self.user_state_tree_root.0.elements[3],
            self.balance,
            self.nonce,
            self.last_checkpoint_id,
            self.event_index,
            self.user_id
        ])
    }
}