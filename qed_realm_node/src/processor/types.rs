use kvq::traits::KVQSerializable;
use qed_core::config::network_constants::QED_CHECKPOINT_JOB_ID_CHANNEL;
use qed_core::job::history_queue::{HistoryQueueMetadata, HistoryQueueMetadataTagged};
use qed_core::job::id::QProvingJobDataID;

#[derive(Debug, Clone, PartialEq)]
pub struct ProvingJobDataId {
    pub checkpoint_id: u64,
    pub job_id: QProvingJobDataID,
}

impl ProvingJobDataId {
    pub fn new(checkpoint_id: u64, job_id: QProvingJobDataID) -> Self {
        Self {
            checkpoint_id,
            job_id,
        }
    }
}

impl HistoryQueueMetadataTagged for ProvingJobDataId {
    fn get_hq_metadata(&self) -> HistoryQueueMetadata {
        HistoryQueueMetadata {
            channel_id: QED_CHECKPOINT_JOB_ID_CHANNEL,
            checkpoint_id: self.checkpoint_id,
            item_id: self.checkpoint_id,
        }
    }
}
impl KVQSerializable for ProvingJobDataId {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut bytes = vec![];
        bytes.extend(self.checkpoint_id.to_le_bytes());
        bytes.extend(self.job_id.to_bytes()?);
        Ok(bytes)
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != 8 + 24 {
            anyhow::bail!("invalid byte length for proving job data id");
        }
        let checkpoint_id = u64::from_le_bytes(bytes[0..8].try_into()?);
        let job_id = QProvingJobDataID::from_bytes(&bytes[8..])?;
        Ok(Self {
            checkpoint_id,
            job_id,
        })
    }
}
