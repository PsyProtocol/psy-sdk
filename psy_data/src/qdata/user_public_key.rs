use kvq::traits::KVQSerializable;
use plonky2::{field::goldilocks_field::GoldilocksField, hash::hash_types::RichField};
use psy_core::data::qhashout::QHashOut;
use serde::{Deserialize, Serialize};
use ts_rs::TS;



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default,TS)]
#[ts(export, concrete(F = GoldilocksField))]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDUserPublicKeyRecord<F: RichField> {
    pub public_key_param: QHashOut<F>,
    pub fingerprint: QHashOut<F>,
    pub public_key: QHashOut<F>,
    pub user_id: u64,
    pub checkpoint_id: u64,
}


impl<F: RichField> KVQSerializable for QEDUserPublicKeyRecord<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}