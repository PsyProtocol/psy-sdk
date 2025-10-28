use plonky2::hash::hash_types::RichField;
use psy_core::data::qhashout::QHashOut;
use serde::{Deserialize, Serialize};



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDZKSignatureCircuitInput<F: RichField> {
    pub private_key: QHashOut<F>,
    pub sig_hash: QHashOut<F>,
}