use async_trait::async_trait;
use kvq::traits::KVQSerializable;
use serde::{Deserialize, Serialize};

pub trait DrainQueueMetadataTagged {
    fn get_dq_metadata(&self) -> DrainQueueMetadata;
}

pub trait DQSerializable: KVQSerializable + DrainQueueMetadataTagged + Send+ Sync {}

impl<T: KVQSerializable + DrainQueueMetadataTagged+ Send+ Sync> DQSerializable for T {}

#[derive(Clone, Debug, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct DrainQueueMetadata {
    pub channel_id: u64,
    pub checkpoint_id: u64,
    pub item_id: u64,
}

impl KVQSerializable for DrainQueueMetadata {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut result = Vec::with_capacity(24);
        result.extend_from_slice(&u64::to_be_bytes(self.channel_id));
        result.extend_from_slice(&u64::to_be_bytes(self.checkpoint_id));
        result.extend_from_slice(&u64::to_be_bytes(self.item_id));

        Ok(result)
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != 24 {
            anyhow::bail!(
                "expected 24 bytes when deserializing DrainQueueMetadata, got {}",
                bytes.len()
            );
        }
        let channel_id = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let checkpoint_id = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        let item_id = u64::from_be_bytes(bytes[16..24].try_into().unwrap());

        Ok(Self {
            channel_id,
            checkpoint_id,
            item_id,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithDrainQueueMetadata<T: KVQSerializable> {
    pub payload: T,
    pub metadata: DrainQueueMetadata,
}
impl<T: KVQSerializable> WithDrainQueueMetadata<T> {
    pub fn new(payload: T, metadata: DrainQueueMetadata) -> Self {
        Self { payload, metadata }
    }
    pub fn new_params(channel_id: u64, checkpoint_id: u64, item_id: u64, payload: T) -> Self {
        Self {
            payload,
            metadata: DrainQueueMetadata {
                channel_id,
                checkpoint_id,
                item_id,
            },
        }
    }
}
impl<T: KVQSerializable> KVQSerializable for WithDrainQueueMetadata<T> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let payload_bytes = self.payload.to_bytes()?;
        let mut result = Vec::with_capacity(payload_bytes.len() + 12);
        result.extend_from_slice(&u64::to_be_bytes(self.metadata.channel_id));
        result.extend_from_slice(&u64::to_be_bytes(self.metadata.checkpoint_id));
        result.extend_from_slice(&u64::to_be_bytes(self.metadata.item_id));
        result.extend_from_slice(&payload_bytes);
        Ok(result)
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() < 24 {
            anyhow::bail!("not enough bytes for deserializing WithDrainQueueMetadata<T>, need at least 24, got {}", bytes.len());
        }

        let channel_id = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let checkpoint_id = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        let item_id = u64::from_be_bytes(bytes[16..24].try_into().unwrap());

        let payload = T::from_bytes(&bytes[24..])?;

        Ok(Self {
            metadata: DrainQueueMetadata {
                channel_id,
                checkpoint_id,
                item_id,
            },
            payload,
        })
    }
}

impl<T: KVQSerializable + Copy> Copy for WithDrainQueueMetadata<T> {}
impl<T: KVQSerializable> DrainQueueMetadataTagged for WithDrainQueueMetadata<T> {
    fn get_dq_metadata(&self) -> DrainQueueMetadata {
        self.metadata
    }
}

#[async_trait]
pub trait CheckpointDrainQueueEmitterAsyncImm {
    async fn cdq_push_imm<T: DQSerializable>(&self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointDrainQueueConsumerAsyncImm {
    async fn cdq_drain_imm<T: DQSerializable>(
        &self,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>>;
    
    /// Peek at items in the queue without consuming them
    /// Returns the items that would be drained by cdq_drain_imm
    async fn cdq_peek_imm<T: DQSerializable>(
        &self,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>>;
}

#[async_trait]
pub trait CheckpointDrainQueueAsyncImm:
    CheckpointDrainQueueEmitterAsyncImm + CheckpointDrainQueueConsumerAsyncImm
{
}

impl<Q: CheckpointDrainQueueEmitterAsyncImm + CheckpointDrainQueueConsumerAsyncImm>
    CheckpointDrainQueueAsyncImm for Q
{
}

#[async_trait]
pub trait CheckpointDrainQueueEmitterAsyncMut {
    async fn cdq_push_mut<T: DQSerializable>(&mut self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointDrainQueueConsumerAsyncMut {
    async fn cdq_drain_mut<T: DQSerializable>(
        &mut self,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>>;
}

#[async_trait]
pub trait CheckpointDrainQueueAsyncMut:
    CheckpointDrainQueueEmitterAsyncMut + CheckpointDrainQueueConsumerAsyncMut
{
}

impl<Q: CheckpointDrainQueueEmitterAsyncMut + CheckpointDrainQueueConsumerAsyncMut>
    CheckpointDrainQueueAsyncMut for Q
{
}

#[async_trait]
pub trait CheckpointDrainQueueEmitterSyncImm {
    fn cdq_push_imm_sync<T: DQSerializable>(&self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointDrainQueueConsumerSyncImm {
    fn cdq_get_imm_sync<T: DQSerializable>(
        &self,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>>;
    fn cdq_drain_imm_sync<T: DQSerializable>(
        &self,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>>;
}

#[async_trait]
pub trait CheckpointDrainQueueSyncImm:
    CheckpointDrainQueueEmitterSyncImm + CheckpointDrainQueueConsumerSyncImm
{
}

impl<Q: CheckpointDrainQueueEmitterSyncImm + CheckpointDrainQueueConsumerSyncImm>
    CheckpointDrainQueueSyncImm for Q
{
}

#[async_trait]
pub trait CheckpointDrainQueueEmitterSyncMut {
    fn cdq_push_mut_sync<T: DQSerializable>(&mut self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointDrainQueueConsumerSyncMut {
    fn cdq_drain_mut_sync<T: DQSerializable>(
        &mut self,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>>;
}

#[async_trait]
pub trait CheckpointDrainQueueSyncMut:
    CheckpointDrainQueueEmitterSyncMut + CheckpointDrainQueueConsumerSyncMut
{
}

impl<Q: CheckpointDrainQueueEmitterSyncMut + CheckpointDrainQueueConsumerSyncMut>
    CheckpointDrainQueueSyncMut for Q
{
}

/*
#[async_trait]
pub trait CheckpointDrainQueueEmitterAsyncImm<T: DQSerializable> {
    async fn cdq_push_imm(&self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointDrainQueueConsumerAsyncImm<T: DQSerializable> {
    async fn cdq_drain_imm(&self, channel_id: u64, checkpoint_id: u64) -> anyhow::Result<Vec<T>>;
}

#[async_trait]
pub trait CheckpointDrainQueueAsyncImm<T: DQSerializable>: CheckpointDrainQueueEmitterAsyncImm<T> + CheckpointDrainQueueConsumerAsyncImm<T>  {
}

impl<T: DQSerializable, Q: CheckpointDrainQueueEmitterAsyncImm<T> + CheckpointDrainQueueConsumerAsyncImm<T>> CheckpointDrainQueueAsyncImm<T> for Q {
}


#[async_trait]
pub trait CheckpointDrainQueueEmitterAsyncMut<T: DQSerializable> {
    async fn cdq_push_mut(&mut self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointDrainQueueConsumerAsyncMut<T: DQSerializable> {
    async fn cdq_drain_mut(&mut self, channel_id: u64, checkpoint_id: u64) -> anyhow::Result<Vec<T>>;
}

#[async_trait]
pub trait CheckpointDrainQueueAsyncMut<T: DQSerializable>: CheckpointDrainQueueEmitterAsyncMut<T> + CheckpointDrainQueueConsumerAsyncMut<T>  {
}

impl<T: DQSerializable, Q: CheckpointDrainQueueEmitterAsyncMut<T> + CheckpointDrainQueueConsumerAsyncMut<T>> CheckpointDrainQueueAsyncMut<T> for Q {
}




#[async_trait]
pub trait CheckpointDrainQueueEmitterSyncImm<T: DQSerializable> {
    fn cdq_push_imm_sync(&self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointDrainQueueConsumerSyncImm<T: DQSerializable> {
    fn cdq_drain_imm_sync(&self, channel_id: u64, checkpoint_id: u64) -> anyhow::Result<Vec<T>>;
}

#[async_trait]
pub trait CheckpointDrainQueueSyncImm<T: DQSerializable>: CheckpointDrainQueueEmitterSyncImm<T> + CheckpointDrainQueueConsumerSyncImm<T>  {
}

impl<T: DQSerializable, Q: CheckpointDrainQueueEmitterSyncImm<T> + CheckpointDrainQueueConsumerSyncImm<T>> CheckpointDrainQueueSyncImm<T> for Q {
}


#[async_trait]
pub trait CheckpointDrainQueueEmitterSyncMut<T: DQSerializable> {
    fn cdq_push_mut_sync(&mut self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointDrainQueueConsumerSyncMut<T: DQSerializable> {
    fn cdq_drain_mut_sync(&mut self, channel_id: u64, checkpoint_id: u64) -> anyhow::Result<Vec<T>>;
}

#[async_trait]
pub trait CheckpointDrainQueueSyncMut<T: DQSerializable>: CheckpointDrainQueueEmitterSyncMut<T> + CheckpointDrainQueueConsumerSyncMut<T>  {
}

impl<T: DQSerializable, Q: CheckpointDrainQueueEmitterSyncMut<T> + CheckpointDrainQueueConsumerSyncMut<T>> CheckpointDrainQueueSyncMut<T> for Q {
}


*/
