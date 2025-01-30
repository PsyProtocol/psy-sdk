use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use serde::{Deserialize, Serialize};

use crate::hash::traits::{hasher::FieldQHasher, qhashable::QFieldHashable};


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QRecursionAggStandardHeader<F: RichField> {
    pub state_transition_start: QHashOut<F>,
    pub state_transition_end: QHashOut<F>,
    pub agg_circuit_whitelist_root: QHashOut<F>,
}

impl<F: RichField> KVQSerializable for QRecursionAggStandardHeader<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}


impl<F: RichField> QFieldHashable<F> for QRecursionAggStandardHeader<F> {
    fn qfhash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {

        let state_combo = H::q_two_to_one(self.state_transition_start, self.state_transition_end);
        H::q_two_to_one(
            self.agg_circuit_whitelist_root,
            state_combo,
        )
    }
}

