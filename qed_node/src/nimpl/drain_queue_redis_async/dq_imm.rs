use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use kvq::traits::KVQPair;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_core::job::drain_queue::{CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm, DQSerializable};
use qed_core::job::id::QProvingJobDataID;
use qed_core::job::traits::{QProofStoreReaderAsync, QProofStoreWriterAsyncImm};
use redis::AsyncCommands;
use std::time::Duration;
use async_trait::async_trait;

pub const PROOFS: &'static str = "proofs";
pub const PROOF_COUNTERS: &'static str = "proof_counters";

#[derive(Clone)]
pub struct DrainQueueRedisAsync {
    pool: Pool<RedisConnectionManager>,
}

impl DrainQueueRedisAsync {
    pub async fn new(uri: &str) -> anyhow::Result<Self> {
        // Create the connection manager
        let manager = RedisConnectionManager::new(uri)?;

        // Build the pool with similar configuration to fred pool
        let pool = Pool::builder()
            .connection_timeout(Duration::from_secs(10))
            .build(manager)
            .await?;

        Ok(Self { pool })
    }

    pub async fn get_connection(&self) -> anyhow::Result<PooledConnection<RedisConnectionManager>> {
        Ok(self.pool.get().await?)
    }

    pub fn get_pool(&self) -> Pool<RedisConnectionManager> {
        self.pool.clone()
    }

    //todo: temporary solution, you should use a suitable queue later
    pub async fn get_imm <T:DQSerializable>(&self, channel_id:u64, checkpoint_id:u64,) -> anyhow::Result<Vec<T> >  {
        let mut conn = self.get_connection().await?;
        let key = format!("CDQ_1_{}_{}",channel_id, checkpoint_id);
        let members: Vec<Vec<u8>> = conn.lrange(key.clone(), 0, -1).await?;

        members.into_iter().map(|x| T::from_bytes(&x)).collect()
    }
}

#[async_trait]
impl CheckpointDrainQueueEmitterAsyncImm for DrainQueueRedisAsync {
   async fn cdq_push_imm<T:DQSerializable>(&self,item:T) -> anyhow::Result<()>  {
        let metadata = item.get_dq_metadata();
        let bytes = item.to_bytes()?;
        let mut conn = self.get_connection().await?;
        conn.lpush::<_, _, ()>(format!("CDQ_1_{}_{}",metadata.channel_id, metadata.checkpoint_id), bytes).await?;

        Ok(())
    }
}

#[async_trait]
impl CheckpointDrainQueueConsumerAsyncImm for DrainQueueRedisAsync {
    async fn cdq_drain_imm<T:DQSerializable>(&self,channel_id:u64,checkpoint_id:u64,) -> anyhow::Result<Vec<T> >  {
        let mut conn = self.get_connection().await?;
        let key = format!("CDQ_1_{}_{}",channel_id, checkpoint_id);
        let members: Vec<Vec<u8>> = conn.lrange(key.clone(), 0, -1).await?;
        conn.del(key).await?;

        members.into_iter().map(|x| T::from_bytes(&x)).collect()
    }
}


#[async_trait]
impl QProofStoreReaderAsync for DrainQueueRedisAsync {
    async fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut conn = self.get_connection().await?;
        let data: Vec<u8> = conn.hget(PROOFS, <[u8; 24]>::from(&id).to_vec()).await?;
        Ok(bincode::deserialize(&data)?)
    }

    async fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let mut conn = self.get_connection().await?;
        let data: Vec<u8> = conn.hget(PROOFS, <[u8; 24]>::from(&id).to_vec()).await?;
        Ok(data)
    }

    async fn contains_id(&self, id: QProvingJobDataID) -> anyhow::Result<bool> {
        Ok(false)
    }
}

#[async_trait]
impl QProofStoreWriterAsyncImm for DrainQueueRedisAsync {
    async fn set_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
    ) -> anyhow::Result<()> {
        let mut conn = self.get_connection().await?;
        conn.hset_nx::<_, _, _, ()>(
            PROOFS,
            <[u8; 24]>::from(&id).to_vec(),
            bincode::serialize(&proof)?,
        ).await?;
        Ok(())
    }

    async fn set_bytes_by_id(&self, id: QProvingJobDataID, data: &[u8]) -> anyhow::Result<()> {
        let mut conn = self.get_connection().await?;
        conn.hset_nx(PROOFS, <[u8; 24]>::from(&id).to_vec(), data).await?;
        Ok(())
    }

    async fn set_bytes_by_id_batch(&self, kv_pairs: &[KVQPair<QProvingJobDataID, Vec<u8>>]) -> anyhow::Result<()> {
        self.set_bytes_by_id_batch_core(kv_pairs).await
    }

    async fn inc_counter_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<u32> {
        let mut conn = self.get_connection().await?;
        let value: u32 = conn.hincr(PROOF_COUNTERS, <[u8; 24]>::from(&id).to_vec(), 1).await?;
        Ok(value)
    }

    async fn write_next_jobs(
        &self,
        jobs: &[QProvingJobDataID],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()> {
        self.write_next_jobs_core(jobs, next_jobs).await
    }

    async fn write_multidimensional_jobs(
        &self,
        jobs_levels: &[Vec<QProvingJobDataID>],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()> {
        self.write_multidimensional_jobs_core(jobs_levels, next_jobs).await
    }
}

