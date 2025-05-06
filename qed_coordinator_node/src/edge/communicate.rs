use std::sync::Arc;
use std::time::Duration;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{error, info, warn};
use kvq::traits::KVQSerializable;
use qed_core::config::network_constants::COORD_STATUS_CHANNEL_ID;
use qed_core::job::drain_queue::{CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm, DQSerializable, DrainQueueMetadata, DrainQueueMetadataTagged};
use qed_node::nimpl::drain_queue_fred::DrainQueueFred;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;

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
    drain_queue: &DrainQueueFred,
) -> anyhow::Result<Option<GlobalCoordinatorStatus>> {
    let checkpoint_id  = 0;
    let entries = drain_queue
        .cdq_get_imm::<GlobalCoordinatorStatus>(COORD_STATUS_CHANNEL_ID, checkpoint_id)
        .await?;

    Ok(entries.into_iter().next())
}

pub async fn push_latest_global_coordinator_status(
    drain_queue: Arc<DrainQueueFred>,
    confirmed_checkpoint_id: u64,
    processor_height: u64,
) -> anyhow::Result<()> {
    info!("Preparing to update global coordinator status, checkpoint_id = {}", confirmed_checkpoint_id);

    let status = GlobalCoordinatorStatus {
        confirmed_checkpoint_id,
        processor_height,
        timestamp: Utc::now().timestamp() as u64,
    };

    drain_queue.cdq_push_imm(status.clone()).await?;
    info!("✅ Status pushed to Database, checkpoint_id = {}", confirmed_checkpoint_id);

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

    Ok(())
}