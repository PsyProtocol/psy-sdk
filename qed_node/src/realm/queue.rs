use fred::interfaces::ListInterface;
use async_trait::async_trait;
use qed_store::queue::proof_store_redis_async::ProofStoreRedisAsync;
use tracing::debug;
use kvq::traits::KVQSerializable;
use qed_core::job::id::ProvingJobDataId;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use fred::prelude::FredResult;
use redis::{AsyncCommands, RedisResult};
use qed_store::queue::proof_store_fred::ProofStoreFred;
use qed_store::queue::rsmq::{QueueId, RsmqQueue};
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use crate::realm::F;

const REAML_PROOF_KEY: &str = "REALM_PROOF";
const REAML_CHECKPOINT_KEY: &str = "REALM_CHECKPOINT";
const PS_SYNC_QUEUE_KEY_PREFIX: &'static str = "PSSQV1";

#[async_trait]
pub trait SyncProofQueue {
    async fn produce_proof(&self, item: ProvingJobDataId) -> anyhow::Result<()>;

    async fn consume_proof(&self) -> anyhow::Result<ProvingJobDataId>;
}

#[async_trait]
impl SyncProofQueue for ProofStoreFred {
    async fn produce_proof(&self, item: ProvingJobDataId) -> anyhow::Result<()> {
        let realm_proof_key = format!("{}-{}", self.worker_queue_id, REAML_PROOF_KEY);
        self.pool()
            .rpush::<(), &str, Vec<u8>>(&realm_proof_key, item.to_bytes()?)
            .await?;
        Ok(())
    }

    async fn consume_proof(&self) -> anyhow::Result<ProvingJobDataId> {
        let realm_proof_key = format!("{}-{}", self.worker_queue_id, REAML_PROOF_KEY);
        let result: FredResult<(String, Vec<u8>)> = self.pool().blpop(realm_proof_key, 0.0).await;

        match result {
            Ok((_, bytes)) => match ProvingJobDataId::from_bytes(&bytes) {
                Ok(id) => Ok(id),
                Err(err) => Err(anyhow::anyhow!(
                    "Failed to parse ProvingJobDataId: {:?}",
                    err
                )),
            },
            Err(err) => Err(anyhow::anyhow!("Error getting job_id from Redis {:?}", err)),
        }
    }
}

#[async_trait]
impl SyncProofQueue for ProofStoreRedisAsync {
    async fn produce_proof(&self, item: ProvingJobDataId) -> anyhow::Result<()> {
        let realm_proof_key = format!("{}-{}", self.worker_queue_id, REAML_PROOF_KEY);
        let mut conn = self.pool().get().await?;
        conn.rpush(&realm_proof_key, item.to_bytes()?)
            .await?;
        Ok(())
    }

    async fn consume_proof(&self) -> anyhow::Result<ProvingJobDataId> {
        let realm_proof_key = format!("{}-{}", self.worker_queue_id, REAML_PROOF_KEY);
        let mut conn = self.pool().get().await?;
        let result: RedisResult<(String, Vec<u8>)> = conn.blpop(realm_proof_key, 0.0).await;

        match result {
            Ok((_, bytes)) => match ProvingJobDataId::from_bytes(&bytes) {
                Ok(id) => Ok(id),
                Err(err) => Err(anyhow::anyhow!(
                    "Failed to parse ProvingJobDataId: {:?}",
                    err
                )),
            },
            Err(err) => Err(anyhow::anyhow!("Error getting job_id from Redis {:?}", err)),
        }
    }
}

#[async_trait]
impl SyncProofQueue for RsmqQueue {
    async fn produce_proof(&self, item: ProvingJobDataId) -> anyhow::Result<()> {
        let queue_id = QueueId::SyncProof {
            worker_queue_suffix: self.worker_queue_suffix.clone(),
        };
        self.send_message(&queue_id, item.to_bytes()?).await
    }

    async fn consume_proof(&self) -> anyhow::Result<ProvingJobDataId> {
        let queue_id = QueueId::SyncProof {
            worker_queue_suffix: self.worker_queue_suffix.clone(),
        };
        match self.pop_message(&queue_id).await? {
            None => {
                anyhow::bail!("No message in queue");
            }
            Some(bytes) => match ProvingJobDataId::from_bytes(&bytes) {
                Ok(id) => Ok(id),
                Err(err) => Err(anyhow::anyhow!(
                    "Failed to parse ProvingJobDataId: {:?}",
                    err
                )),
            },
        }
    }
}

#[async_trait]
pub trait SyncCheckpointQueue {
    async fn produce_checkpoint_async_info(
        &self,
        item: CheckpointSyncInfo<F>,
    ) -> anyhow::Result<()>;

    async fn is_empty(&self) -> anyhow::Result<bool>;

    async fn consume_checkpoint_async_info(&self) -> anyhow::Result<CheckpointSyncInfo<F>>;
}

#[derive(Clone, Debug)]
pub struct Queue {
    pool: Pool<RedisConnectionManager>,
    worker_queue_id: String,
}

impl Queue {
    pub async fn new(url: &str, pool_size: usize, worker_queue_suffix:String) -> anyhow::Result<Self> {
        let manager = RedisConnectionManager::new(url)?;
        let pool = Pool::builder().
            max_size(pool_size as u32).
            build(manager).await?;

        Ok(Self { pool ,worker_queue_id:format!("{}-{}", PS_SYNC_QUEUE_KEY_PREFIX, worker_queue_suffix)})
    }

    pub fn pool(&self) -> Pool<RedisConnectionManager> {
        self.pool.clone()
    }

    pub fn worker_queue_id(&self) -> String {
        self.worker_queue_id.clone()
    }

    pub fn realm_checkpoint_key(&self) -> String {
        format!("{}-{}", self.worker_queue_id, REAML_CHECKPOINT_KEY)
    }
}

#[async_trait]
impl SyncCheckpointQueue for Queue {
    async fn produce_checkpoint_async_info(
        &self,
        item: CheckpointSyncInfo<F>,
    ) -> anyhow::Result<()> {
        debug!("Producing checkpoint async info to Redis: checkpoint_id before: {}", item.compact.l2_block_state.checkpoint_id);
        let mut conn = self.pool.get().await?;
        conn.rpush(self.realm_checkpoint_key().as_str(), item.to_bytes()?)
            .await?;

        debug!("Checkpoint async info produced to Redis: checkpoint_id after: {}", item.compact.l2_block_state.checkpoint_id);
        Ok(())
    }

    async fn is_empty(&self) -> anyhow::Result<bool> {
        let mut conn = self.pool.get().await?;
        let length: u64 = conn.llen(self.realm_checkpoint_key()).await?;
        Ok(length == 0)
    }

    async fn consume_checkpoint_async_info(&self) -> anyhow::Result<CheckpointSyncInfo<F>> {
        let mut conn = self.pool.get().await?;
        let result: RedisResult<(String, Vec<u8>)> = conn.blpop(self.realm_checkpoint_key().as_str(), 0.0).await;

        match result {
            Ok((_, bytes)) => match CheckpointSyncInfo::from_bytes(&bytes) {
                Ok(info) => Ok(info),
                Err(err) => Err(anyhow::anyhow!(
                    "Failed to parse CheckpointSyncInfo: {:?}",
                    err
                )),
            },
            Err(err) => Err(anyhow::anyhow!(
                "Error getting checkpoint info from Redis {:?}",
                err
            )),
        }
    }
}