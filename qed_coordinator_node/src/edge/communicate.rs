use std::sync::Arc;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use kvq::traits::KVQSerializable;
use qed_core::config::network_constants::COORD_STATUS_CHANNEL_ID;
use qed_core::job::drain_queue::{CheckpointDrainQueueConsumerSyncImm, CheckpointDrainQueueEmitterSyncImm, DrainQueueMetadata, DrainQueueMetadataTagged};
use qed_node::nimpl::drain_queue_redis::dq_imm::DrainQueueRedis;

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GlobalCoordinatorStatus {
    pub confirmed_checkpoint_id: u64,//worker have confirmed
    pub processor_height: u64, //height of the processor
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


pub async fn get_latest_global_coordinator_status(
    drain_queue: &DrainQueueRedis,
) -> anyhow::Result<Option<GlobalCoordinatorStatus>> {
    let checkpoint_id  = 0;
    let entries = drain_queue
        .cdq_get_imm_sync::<GlobalCoordinatorStatus>(COORD_STATUS_CHANNEL_ID, checkpoint_id)?;

    Ok(entries.into_iter().next())
}

pub async fn push_latest_global_coordinator_status(
    drain_queue: Arc<DrainQueueRedis>,
    confirmed_checkpoint_id: u64,
    processor_height: u64,
) {
    // info!("Preparing to update global coordinator status, checkpoint_id = {}", confirmed_checkpoint_id);

    let status = GlobalCoordinatorStatus {
        confirmed_checkpoint_id,
        processor_height,
        timestamp: Utc::now().timestamp() as u64,
    };

    match drain_queue.cdq_push_imm_sync(status.clone()) {
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
    // info!("✅ Status pushed to Database, checkpoint_id = {}", confirmed_checkpoint_id);

    // sleep(Duration::from_millis(500)).await;
    //
    // match get_latest_global_coordinator_status(&drain_queue).await? {
    //     Some(fetched) => {
    //         info!("🔍 Redis returned latest GlobalCoordinatorStatus:");
    //         info!("🟢 confirmed_checkpoint_id = {}", fetched.confirmed_checkpoint_id);
    //         info!("🟢 processor_height = {}", fetched.processor_height);
    //         info!("🕒 sync_timestamp = {}", fetched.timestamp);
    //
    //         if fetched != status {
    //             warn!("⚠️  Mismatch between pushed and fetched status!");
    //             warn!("📤 pushed:   {:?}", status);
    //             warn!("📥 fetched:  {:?}", fetched);
    //         } else {
    //             info!("🎉 Pushed and fetched status match!");
    //         }
    //     }
    //     None => {
    //         warn!("⚠️ Redis returned no coordinator status after push.");
    //     }
    // }
}