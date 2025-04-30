use std::sync::Arc;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use kvq::traits::KVQSerializable;
use qed_core::config::network_constants::COORD_STATUS_CHANNEL_ID;
use qed_core::job::drain_queue::{CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm, DQSerializable, DrainQueueMetadata, DrainQueueMetadataTagged};
use qed_node::nimpl::drain_queue_fred::DrainQueueFred;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            checkpoint_id: self.confirmed_checkpoint_id, //use 0 for global status
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
    let status = GlobalCoordinatorStatus {
        confirmed_checkpoint_id,
        processor_height,
        timestamp: Utc::now().timestamp() as u64,
    };

    drain_queue.cdq_push_imm(status.clone()).await?;

    {
        let entries = drain_queue
            .cdq_get_imm::<GlobalCoordinatorStatus>(COORD_STATUS_CHANNEL_ID, 0)
            .await?;

        if let Some(latest) = entries.first() {
            if latest == &status {
                info!("✅ push & get match: {:?}", latest);
            } else {
                error!("❌ push & get mismatch:\n  pushed: {:?}\n  fetched: {:?}", status, latest);
            }
        } else {
            error!("❌ push succeeded but no value found at checkpoint_id = 0");
        }

    }
    Ok(())
}