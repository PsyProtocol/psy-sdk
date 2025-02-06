use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use serde::{Deserialize, Serialize};



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct GUTAStats<F: RichField> {
    pub fees_collected: F,

    pub user_ops_processed: F,
    pub total_transactions: F,

    pub slots_modified: F,
}

impl<F: RichField> KVQSerializable for GUTAStats<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}