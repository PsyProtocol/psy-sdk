use serde::{Deserialize, Serialize};
use qed_data::qdata::checkpoint::PendingCheckpointState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestCheckpointResponse {
    pub checkpoint_id: u64,
    pub pending_checkpoint_state: Option<PendingCheckpointState>,
}
