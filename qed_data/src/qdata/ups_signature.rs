use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::{
    data::qhashout::QHashOut,
    traits::to_qfelts::QFeltSized,
};
use qed_crypto::hash::traits::{
    hasher::FieldQHasher,
    qhashable::QFieldHashable,
};
use serde::{Deserialize, Serialize};



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDUserProvingSessionSignatureDataCompact<F: RichField> {
    pub start_user_leaf_hash: QHashOut<F>,
    pub end_user_leaf_hash: QHashOut<F>,
    pub checkpoint_leaf_hash: QHashOut<F>,
    pub tx_stack_hash: QHashOut<F>,
    pub tx_count: F,
}

impl<F: RichField> KVQSerializable for QEDUserProvingSessionSignatureDataCompact<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

impl<F: RichField> QFeltSized for QEDUserProvingSessionSignatureDataCompact<F> {
    fn q_felt_size() -> usize {
        17
    }
}

impl<F: RichField> QFieldHashable<F> for QEDUserProvingSessionSignatureDataCompact<F> {
    fn qfhash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {
        let user_leaf_change_combo = H::q_two_to_one(
            self.start_user_leaf_hash,
            self.end_user_leaf_hash,
        );
        let tx_sized_hash = H::q_hash_many(&[
            self.tx_count,
            self.tx_stack_hash.0.elements[0],
            self.tx_stack_hash.0.elements[1],
            self.tx_stack_hash.0.elements[2],
            self.tx_stack_hash.0.elements[3],
        ]);

        let state_context_combo = H::q_two_to_one(
            self.checkpoint_leaf_hash,
            user_leaf_change_combo,
        );

        H::q_two_to_one(
            state_context_combo,
            tx_sized_hash
        )
    }
}


