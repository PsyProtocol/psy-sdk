use std::time::Duration;

use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::{AsyncCommands, RedisResult};
use kvq::traits::KVQSerializable;
use qed_core::job::id::ProvingJobDataId;
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use plonky2::field::goldilocks_field::GoldilocksField;
use tracing::debug;

use async_trait::async_trait;
use auto_impl::auto_impl;
use kvq::traits::KVQPair;
use plonky2::plonk::{config::GenericConfig, proof::ProofWithPublicInputs};
use qed_core::job::{
    drain_queue::{
        CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm, DQSerializable,
    },
    history_queue::{
        CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm,
        HQSerializable,
    },
    id::QProvingJobDataID,
    traits::{QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
    worker_queue::{WorkerEventReceiverAsyncImm, WorkerEventTransmitterAsyncImm},
};
use tokio::time::sleep;

// Re-use constants from fred_queue
use crate::queue::fred_queue::{
    PROOF_STORE_COUNTERS_PREFIX_1, PROOF_STORE_KEY_PREFIX_1, PS_DRAIN_QUEUE_KEY_PREFIX,
    PS_HISTORY_QUEUE_KEY_PREFIX, PS_NOTIFICATIONS_QUEUE_KEY_PREFIX, PS_WORKER_QUEUE_KEY_PREFIX,
    REAML_PROOF_KEY, SyncProofQueue,
};

#[derive(Debug, Clone)]
pub struct ProofStoreRedisAsync {
    pool: Pool<RedisConnectionManager>,
    pub worker_queue_id: String,
    notifications_queue_id: String,
    proof_store_key: String,
    proof_store_counters: String,
}

impl ProofStoreRedisAsync {
    pub async fn new2(
        pool: Pool<RedisConnectionManager>,
        worker_queue_suffix: &str,
        notifications_queue_suffix: &str,
        proof_store_key_suffix: &str,
        proof_store_counters_suffix: &str,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            pool,
            worker_queue_id: format!("{}-{}", PS_WORKER_QUEUE_KEY_PREFIX, worker_queue_suffix),
            notifications_queue_id: format!(
                "{}-{}",
                PS_NOTIFICATIONS_QUEUE_KEY_PREFIX, notifications_queue_suffix
            ),
            proof_store_key: format!("{}-{}", PROOF_STORE_KEY_PREFIX_1, proof_store_key_suffix),
            proof_store_counters: format!(
                "{}-{}",
                PROOF_STORE_COUNTERS_PREFIX_1, proof_store_counters_suffix
            ),
        })
    }
    pub fn pool(&self) -> &Pool<RedisConnectionManager> {
        &self.pool
    }
}

#[async_trait]
impl QProofStoreReaderAsync for ProofStoreRedisAsync {
    async fn contains_id(&self, id: QProvingJobDataID) -> anyhow::Result<bool> {
        let mut con = self.pool.get().await?;
        let data: Vec<u8> = con
            .hget(&self.proof_store_key, id.to_fixed_bytes().as_slice())
            .await?;
        Ok(!data.is_empty())
    }
    async fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!(?id, "Getting proof by id");
        let mut con = self.pool.get().await?;
        let data: Vec<u8> = con
            .hget(&self.proof_store_key, id.to_fixed_bytes().as_slice())
            .await?;
        tracing::info!(?id, "Got proof by id, data.len = {}", data.len());
        Ok(bincode::deserialize(&data)?)
    }

    async fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let mut con = self.pool.get().await?;
        let data: Vec<u8> = con
            .hget(&self.proof_store_key, id.to_fixed_bytes().as_slice())
            .await?;
        Ok(data)
    }
}

#[async_trait]
impl QProofStoreWriterAsyncImm for ProofStoreRedisAsync {
    async fn set_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
    ) -> anyhow::Result<()> {
        tracing::info!(?id, "Setting proof by id");
        let data = bincode::serialize(&proof).unwrap();

        let mut con = self.pool.get().await?;
        let _: bool = con.hset_nx(
            &self.proof_store_key,
            id.to_fixed_bytes().as_slice(),
            data.as_slice(),
        )
        .await
        .unwrap();

        Ok(())
    }
    async fn set_bytes_by_id_batch(
        &self,
        kv_pairs: &[KVQPair<QProvingJobDataID, Vec<u8>>],
    ) -> anyhow::Result<()> {
        self.set_bytes_by_id_batch_core(kv_pairs).await
    }
    async fn set_bytes_by_id(&self, id: QProvingJobDataID, data: &[u8]) -> anyhow::Result<()> {
        tracing::info!(?id, "Setting bytes by id, data.len = {}", data.len());
        let mut con = self.pool.get().await?;
        let _: bool = con.hset_nx(&self.proof_store_key, id.to_fixed_bytes().as_slice(), data)
            .await?;
        Ok(())
    }

    async fn inc_counter_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<u32> {
        let mut con = self.pool.get().await?;
        let new_counter_value: u32 = con
            .hincr(
                &self.proof_store_counters,
                id.to_fixed_bytes().as_slice(),
                1,
            )
            .await?;
        Ok(new_counter_value)
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
        self.write_multidimensional_jobs_core(jobs_levels, next_jobs)
            .await
    }
}

#[async_trait]
impl CheckpointDrainQueueEmitterAsyncImm for ProofStoreRedisAsync {
    async fn cdq_push_imm<T: DQSerializable>(&self, item: T) -> anyhow::Result<()> {
        let checkpoint_queue_prefix =
            format!("{}-{}", self.worker_queue_id, PS_DRAIN_QUEUE_KEY_PREFIX);
        let metadata: qed_core::job::drain_queue::DrainQueueMetadata = item.get_dq_metadata();
        let bytes = item.to_bytes()?;
        let key = format!(
            "{}-{}",
            checkpoint_queue_prefix, metadata.channel_id,
        );
        tracing::debug!("Pushing job id to queue: {:?}", key);
        let mut con = self.pool.get().await?;
        con.lpush(key, bytes).await?;

        Ok(())
    }
}

#[async_trait]
impl CheckpointDrainQueueConsumerAsyncImm for ProofStoreRedisAsync {
    async fn cdq_drain_imm<T: DQSerializable>(
        &self,
        channel_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let checkpoint_queue_prefix =
            format!("{}-{}", self.worker_queue_id, PS_DRAIN_QUEUE_KEY_PREFIX);
        let key = format!(
            "{}-{}",
            checkpoint_queue_prefix, channel_id
        );
        let mut con = self.pool.get().await?;
        let members: Vec<Vec<u8>> = con.lrange(key.clone(), 0, -1).await?;
        con.del(key).await?;

        members
            .into_iter()
            .rev()
            .map(|x| T::from_bytes(&x))
            .collect()
    }

    async fn cdq_peek_imm<T: DQSerializable>(
        &self,
        channel_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let checkpoint_queue_prefix =
            format!("{}-{}", self.worker_queue_id, PS_DRAIN_QUEUE_KEY_PREFIX);
        let key = format!(
            "{}-{}",
            checkpoint_queue_prefix, channel_id
        );
        let mut con = self.pool.get().await?;
        let members: Vec<Vec<u8>> = con.lrange(key, 0, -1).await?;
        members
            .into_iter()
            .rev()
            .map(|x| T::from_bytes(&x))
            .collect()
    }
}

#[async_trait]
impl WorkerEventReceiverAsyncImm for ProofStoreRedisAsync {
    async fn wait_for_next_job_imm(&self) -> anyhow::Result<QProvingJobDataID> {
        loop {
            let mut con = self.pool.get().await?;
            let job_res: Option<Vec<u8>> = con.lpop(&self.worker_queue_id, None).await?;
            match job_res {
                Some(g) => {
                    if g.len() == 24 {
                        return Ok(QProvingJobDataID::try_from_byte_vec(&g)?);
                    }
                }
                None => {}
            };
            sleep(Duration::from_millis(100)).await;
        }
    }
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        con.lpush(
            &self.worker_queue_id,
            jobs.iter().map(|x| x.to_fixed_bytes().to_vec()).collect::<Vec<Vec<u8>>>().as_slice(),
        )
        .await?;

        Ok(())
    }
    async fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        con.lpush(&self.notifications_queue_id, job.to_fixed_bytes().as_slice())
            .await?;

        Ok(())
    }
}

#[async_trait]
impl WorkerEventTransmitterAsyncImm for ProofStoreRedisAsync {
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        con.lpush(
            &self.worker_queue_id,
            jobs.iter().map(|x| x.to_fixed_bytes().to_vec()).collect::<Vec<Vec<u8>>>().as_slice(),
        )
        .await?;

        Ok(())
    }
    async fn wait_for_block_proving_jobs_imm(
        &self,
        _checkpoint_id: u64,
    ) -> anyhow::Result<QProvingJobDataID> {
        loop {
            let mut con = self.pool.get().await?;
            let job_res: Option<Vec<u8>> = con.lpop(&self.notifications_queue_id, None).await?;
            match job_res {
                Some(g) => {
                    if g.len() == 24 {
                        match QProvingJobDataID::try_from_byte_vec(&g) {
                            Ok(job) => {
                                if job.is_notify_complete() {
                                    return Ok(job);
                                }
                            }
                            Err(e1) => eprintln!(
                                "error deserializing job id in wait_for_block_proving_jobs_imm: {:?}",
                                e1
                            ),
                        }
                    }
                }
                None => {}
            };
            sleep(Duration::from_millis(500)).await;
        }
    }
}

#[async_trait]
impl CheckpointHistoryQueueEmitterAsyncImm for ProofStoreRedisAsync {
    async fn chq_push_imm<T: HQSerializable>(&self, item: T) -> anyhow::Result<()> {
        let metadata = item.get_hq_metadata();
        let bytes = item.to_bytes()?;
        let mut con = self.pool.get().await?;
        con.set(
            format!(
                "{}-{}_{}",
                PS_HISTORY_QUEUE_KEY_PREFIX, metadata.channel_id, metadata.checkpoint_id,
            ),
            bytes.as_slice(),
        )
        .await?;
        con.set(
            format!("{}-{}", PS_HISTORY_QUEUE_KEY_PREFIX, metadata.channel_id,),
            metadata.checkpoint_id,
        )
        .await?;
        Ok(())
    }
}

#[async_trait]
impl CheckpointHistoryQueueConsumerAsyncImm for ProofStoreRedisAsync {
    async fn chq_listen_from_imm<T: HQSerializable>(
        &self,
        channel_id: u64,
        start_checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let mut con = self.pool.get().await?;
        let cur_checkpoint_id_opt: Option<u64> = con
            .get(format!("{}-{}", PS_HISTORY_QUEUE_KEY_PREFIX, channel_id,))
            .await?;
        match cur_checkpoint_id_opt {
            Some(r) => {
                if r >= start_checkpoint_id {
                    let mut results = Vec::with_capacity((r - start_checkpoint_id + 1) as usize);

                    for i in (start_checkpoint_id..=r) {
                        let result: Vec<u8> = con
                            .get(format!(
                                "{}-{}_{}",
                                PS_HISTORY_QUEUE_KEY_PREFIX, channel_id, i,
                            ))
                            .await?;
                        results.push(T::from_bytes(&result)?);
                    }

                    Ok(results)
                } else {
                    Ok(Vec::new())
                }
            }
            None => Ok(Vec::new()),
        }
    }
    async fn wait_for_next_item_imm<T: HQSerializable>(
        &self,
        channel_id: u64,
        start_checkpoint_id: u64,
    ) -> anyhow::Result<T> {
        let start_i64 = start_checkpoint_id as i64;
        let mut checkpoint_current: i64 = -1;

        while checkpoint_current < start_i64 {
            sleep(Duration::from_millis(100)).await;
            let mut con = self.pool.get().await?;
            let cur_checkpoint_id_opt: Option<u64> = con
                .get(format!("{}-{}", PS_HISTORY_QUEUE_KEY_PREFIX, channel_id,))
                .await?;
            checkpoint_current = match cur_checkpoint_id_opt {
                Some(x) => x as i64,
                None => -1,
            };
        }
        let mut con = self.pool.get().await?;
        let result: Vec<u8> = con
            .get(format!(
                "{}-{}_{}",
                PS_HISTORY_QUEUE_KEY_PREFIX, channel_id, checkpoint_current,
            ))
            .await?;
        Ok(T::from_bytes(&result)?)
    }
    
    async fn is_empty(&self) -> anyhow::Result<bool> {
        // Check if REALM_CHECKPOINT queue is empty
        let realm_checkpoint_key = format!("{}-REALM_CHECKPOINT", self.worker_queue_id);
        let mut conn = self.pool.get().await?;
        let length: u64 = conn.llen(&realm_checkpoint_key).await?;
        Ok(length == 0)
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

// Checkpoint queue types
pub type F = GoldilocksField;

pub const REAML_CHECKPOINT_KEY: &str = "REALM_CHECKPOINT";
pub const PS_SYNC_QUEUE_KEY_PREFIX: &'static str = "PSSQV1";

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
    pub async fn new(url: &str, pool_size: usize, worker_queue_suffix: String) -> anyhow::Result<Self> {
        let manager = RedisConnectionManager::new(url)?;
        let pool = Pool::builder()
            .max_size(pool_size as u32)
            .build(manager).await?;

        Ok(Self { 
            pool,
            worker_queue_id: format!("{}-{}", PS_SYNC_QUEUE_KEY_PREFIX, worker_queue_suffix),
        })
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

#[async_trait]
#[auto_impl(Box, Arc)]
pub trait NotificationQueue<T: HQSerializable> {
    async fn produce_item(
        &self,
        item: T,
    ) -> anyhow::Result<()> where T: 'async_trait;
    async fn consume_item(&self, channel_id: u64) -> anyhow::Result<T>;
}

#[async_trait]
impl<T: HQSerializable> NotificationQueue<T> for ProofStoreRedisAsync {
    async fn produce_item(&self, item: T) -> anyhow::Result<()> where T: 'async_trait {
        let mut conn = self.pool.get().await?;
        let key = format!("{}-{}", self.notifications_queue_id, item.get_hq_metadata().channel_id);
        conn.rpush(key.as_str(), item.to_bytes()?).await?;
        Ok(())
    }

    async fn consume_item(&self, channel_id: u64) -> anyhow::Result<T> {

        loop {
            let mut conn = self.pool.get().await?;
            let key = format!("{}-{}", self.notifications_queue_id, channel_id);
            let result: Option<Vec<u8>> = conn.lpop(key.as_str(), None).await?;
            match result {
                Some(result) => {
                    match T::from_bytes(&result) {
                        Ok(item) => return Ok(item),
                        Err(err) => return Err(anyhow::anyhow!("Failed to parse item: {:?}", err)),
                    }
                }
                None => {
                    sleep(Duration::from_millis(200)).await;
                }
            }
        }
    }
}