use std::sync::Arc;

use chrono::Utc;
use kvq::traits::KVQSerializable;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use qed_core::config::network_constants::COORD_STATUS_CHANNEL_ID;
use qed_core::job::drain_queue::{
    CheckpointDrainQueueEmitterAsyncImm, DrainQueueMetadata, DrainQueueMetadataTagged,
};
use qed_node::nimpl::drain_queue_redis_async::dq_imm::DrainQueueRedisAsync;

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GlobalCoordinatorStatus {
    pub confirmed_checkpoint_id: u64, //worker have confirmed
    pub processor_height: u64,        //height of the processor
    pub timestamp: u64,
}

impl KVQSerializable for GlobalCoordinatorStatus {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(bincode::deserialize(bytes)?)
    }
}
impl DrainQueueMetadataTagged for GlobalCoordinatorStatus {
    fn get_dq_metadata(&self) -> DrainQueueMetadata {
        DrainQueueMetadata {
            channel_id: COORD_STATUS_CHANNEL_ID,
            checkpoint_id: 0, //use 0 for global status
            item_id: 0,
        }
    }
}

pub async fn push_latest_global_coordinator_status(
    drain_queue: Arc<DrainQueueRedisAsync>,
    confirmed_checkpoint_id: u64,
    processor_height: u64,
) {
    let status = GlobalCoordinatorStatus {
        confirmed_checkpoint_id,
        processor_height,
        timestamp: Utc::now().timestamp() as u64,
    };

    match drain_queue.cdq_push_imm(status.clone()).await {
        Ok(_) => {
            info!(
                "⭐ Updated confirmed checkpoint id = {}, processor height = {}",
                confirmed_checkpoint_id, processor_height
            );
        }
        Err(e) => {
            error!(
                "❌ Failed to update confirmed coordinator status (checkpoint_id = {}, processor_height = {}): {:?}",
                confirmed_checkpoint_id, processor_height, e
            );
        }
    }
}
