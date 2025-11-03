use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestCheckpointResponse {
    pub checkpoint_id: u64,
}
