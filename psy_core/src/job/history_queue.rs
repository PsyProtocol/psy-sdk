use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use async_trait::async_trait;
use kvq::traits::KVQSerializable;
use serde::{Deserialize, Serialize};

pub trait HistoryQueueMetadataTagged {
    fn get_hq_metadata(&self) -> HistoryQueueMetadata;
}

pub trait HQSerializable: KVQSerializable + HistoryQueueMetadataTagged + Send+ Sync {}

impl<T: KVQSerializable + HistoryQueueMetadataTagged+ Send+ Sync> HQSerializable for T {}

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct HistoryQueueMetadata {
    pub channel_id: u64,
    pub checkpoint_id: u64,
    pub item_id: u64,
}

impl KVQSerializable for HistoryQueueMetadata {
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
                "expected 24 bytes when deserializing HistoryQueueMetadata, got {}",
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
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct WithHistoryQueueMetadata<T: KVQSerializable> {
    pub payload: T,
    pub metadata: HistoryQueueMetadata,
}
impl<T: KVQSerializable> WithHistoryQueueMetadata<T> {
    pub fn new(payload: T, metadata: HistoryQueueMetadata) -> Self {
        Self { payload, metadata }
    }
    pub fn new_params(channel_id: u64, checkpoint_id: u64, item_id: u64, payload: T) -> Self {
        Self {
            payload,
            metadata: HistoryQueueMetadata {
                channel_id,
                checkpoint_id,
                item_id,
            },
        }
    }
}
impl<T: KVQSerializable> KVQSerializable for WithHistoryQueueMetadata<T> {
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
            anyhow::bail!("not enough bytes for deserializing WithHistoryQueueMetadata<T>, need at least 24, got {}", bytes.len());
        }

        let channel_id = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let checkpoint_id = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        let item_id = u64::from_be_bytes(bytes[16..24].try_into().unwrap());

        let payload = T::from_bytes(&bytes[24..])?;

        Ok(Self {
            metadata: HistoryQueueMetadata {
                channel_id,
                checkpoint_id,
                item_id,
            },
            payload,
        })
    }
}

impl<T: KVQSerializable + Copy> Copy for WithHistoryQueueMetadata<T> {}
impl<T: KVQSerializable> HistoryQueueMetadataTagged for WithHistoryQueueMetadata<T> {
    fn get_hq_metadata(&self) -> HistoryQueueMetadata {
        self.metadata
    }
}

#[async_trait]
pub trait CheckpointHistoryQueueEmitterAsyncImm {
    async fn chq_push_imm<T: HQSerializable>(&self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointHistoryQueueConsumerAsyncImm {
    async fn chq_items_gte<T: HQSerializable>(
        &self,
        channel_id: u64,
        start_checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>>;
    async fn wait_for_next_item_imm<T: HQSerializable>(
        &self,
        channel_id: u64,
        start_checkpoint_id: u64,
    ) -> anyhow::Result<T>;
}

#[async_trait]
pub trait CheckpointHistoryQueueAsyncImm:
    CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm
{
}

impl<Q: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm>
    CheckpointHistoryQueueAsyncImm for Q
{
}

#[async_trait]
pub trait CheckpointHistoryQueueEmitterAsyncMut {
    async fn chq_push_mut<T: HQSerializable>(&mut self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointHistoryQueueConsumerAsyncMut {
    async fn chq_items_gte<T: HQSerializable>(
        &mut self,
        channel_id: u64,
        start_checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>>;

    async fn wait_for_next_item_mut<T: HQSerializable>(
        &mut self,
        channel_id: u64,
        start_checkpoint_id: u64,
    ) -> anyhow::Result<T>;
}

#[async_trait]
pub trait CheckpointHistoryQueueAsyncMut:
    CheckpointHistoryQueueEmitterAsyncMut + CheckpointHistoryQueueConsumerAsyncMut
{
}

impl<Q: CheckpointHistoryQueueEmitterAsyncMut + CheckpointHistoryQueueConsumerAsyncMut>
    CheckpointHistoryQueueAsyncMut for Q
{
}

#[async_trait]
pub trait CheckpointHistoryQueueEmitterSyncImm {
    fn chq_push_imm_sync<T: HQSerializable>(&self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointHistoryQueueConsumerSyncImm {
    fn chq_drain_imm_sync<T: HQSerializable>(
        &self,
        channel_id: u64,
    ) -> anyhow::Result<Vec<T>>;
}

#[async_trait]
pub trait CheckpointHistoryQueueSyncImm:
    CheckpointHistoryQueueEmitterSyncImm + CheckpointHistoryQueueConsumerSyncImm
{
}

impl<Q: CheckpointHistoryQueueEmitterSyncImm + CheckpointHistoryQueueConsumerSyncImm>
    CheckpointHistoryQueueSyncImm for Q
{
}

#[async_trait]
pub trait CheckpointHistoryQueueEmitterSyncMut {
    fn chq_push_mut_sync<T: HQSerializable>(&mut self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CheckpointHistoryQueueConsumerSyncMut {
    fn chq_drain_mut_sync<T: HQSerializable>(
        &mut self,
        channel_id: u64,
    ) -> anyhow::Result<Vec<T>>;
}

#[async_trait]
pub trait CheckpointHistoryQueueSyncMut:
    CheckpointHistoryQueueEmitterSyncMut + CheckpointHistoryQueueConsumerSyncMut
{
}

impl<Q: CheckpointHistoryQueueEmitterSyncMut + CheckpointHistoryQueueConsumerSyncMut>
    CheckpointHistoryQueueSyncMut for Q
{
}




#[derive(Clone)]
pub struct QEDArcImmutableHistoryQueueWrapper<P> {
    pub inner: Arc<RwLock<P>>,
}
impl<P> QEDArcImmutableHistoryQueueWrapper<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }
    pub fn dup(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }

    pub fn write(&self) -> anyhow::Result<RwLockWriteGuard<P>> {
        self.inner
            .try_write()
            .map_err(|err| anyhow::anyhow!("Error writing to immutable store: {:?}", err))
    }
    pub fn read(&self) -> anyhow::Result<RwLockReadGuard<P>> {
        self.inner
            .try_read()
            .map_err(|err| anyhow::anyhow!("Error reading from immutable store: {:?}", err))
    }
}