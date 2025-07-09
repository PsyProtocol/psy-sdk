use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;
use qed_data::config::store_config::QEDFelt;
use serde::{Deserialize, Serialize};

/// push the latest checkpoint sync info
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct CheckpointSyncInfo<F: RichField> {
    pub latest_checkpoint_id: u64, // latest checkpoint id
    pub description: Option<String>,
    pub source_coordinator_edge_id: Option<String>,
    pub sync_timestamp: u64, // sync timestamp
    pub compact: QEDCheckpointSyncInfoCompact<F>,
}

impl<F: RichField + Serialize + for<'de> Deserialize<'de>> KVQSerializable
    for CheckpointSyncInfo<F>
{
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

