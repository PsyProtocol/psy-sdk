use std::{sync::Arc, time::Duration};

use auto_impl::auto_impl;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::{AsyncCommands, HashFieldExpirationOptions, SetExpiry};
use qed_core::job::id::{JobsTask, JobsTaskGraph};

use async_trait::async_trait;
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
use tokio::{sync::Mutex, time::sleep};
// Re-use constants from fred_queue
use crate::queue::fred_queue::{PROOF_STORE_COUNTERS_PREFIX_1, PROOF_STORE_KEY_PREFIX_1, PS_DRAIN_QUEUE_KEY_PREFIX, PS_HISTORY_QUEUE_KEY_PREFIX, PS_NOTIFICATIONS_QUEUE_KEY_PREFIX, PS_WORKER_QUEUE_KEY_PREFIX};

#[auto_impl(&, Box, Arc)]
pub trait BizKey {
    fn biz_key(&self) -> String;
}


pub trait QueuePrefixKey {
    fn worker_queue_key(&self) -> String;
    fn notifications_queue_key(&self) -> String;
    fn proof_store_key(&self) -> String;
    fn proof_store_counters_key(&self) -> String;

    // checkpoint history queue key prefix PS_HISTORY_QUEUE_KEY_PREFIX
    fn checkpoint_history_queue_prefix_key(&self) -> String;

    fn checkpoint_drain_queue_key(&self) -> String;
}

// fixed prefix + biz key
impl<T: BizKey> QueuePrefixKey for T {

    fn worker_queue_key(&self) -> String {
        format!("{}-{}", PS_WORKER_QUEUE_KEY_PREFIX, self.biz_key())
    }

    fn notifications_queue_key(&self) -> String {
        format!("{}-{}", PS_NOTIFICATIONS_QUEUE_KEY_PREFIX, self.biz_key())
    }

    fn proof_store_key(&self) -> String {
        format!("{}-{}", PROOF_STORE_KEY_PREFIX_1, self.biz_key())
    }

    fn proof_store_counters_key(&self) -> String {
        format!("{}-{}", PROOF_STORE_COUNTERS_PREFIX_1, self.biz_key())
    }

    fn checkpoint_history_queue_prefix_key(&self) -> String {
        format!("{}-{}", PS_HISTORY_QUEUE_KEY_PREFIX, self.biz_key())
    }

    fn checkpoint_drain_queue_key(&self) -> String {
        format!("CDQ_2_{}", self.biz_key())
    }
}

#[derive(Debug, Clone)]
pub struct ProofStoreRedisAsync {
    pool: Pool<RedisConnectionManager>,
    pub task_graph: Arc<Mutex<JobsTaskGraph>>,
    biz_key: String,
}

impl BizKey for ProofStoreRedisAsync {
    fn biz_key(&self) -> String {
        self.biz_key.clone()
    }
}

impl ProofStoreRedisAsync {
    pub async fn new(
        pool: Pool<RedisConnectionManager>,
        biz_key: String,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            pool,
            task_graph: Arc::new(Mutex::new(JobsTaskGraph::new())),
            biz_key: biz_key,
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
            .hget(&self.proof_store_key(), id.to_fixed_bytes().as_slice())
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
            .hget(&self.proof_store_key(), id.to_fixed_bytes().as_slice())
            .await?;
        tracing::info!(?id, "Got proof by id, data.len = {}", data.len());
        Ok(bincode::deserialize(&data)?)
    }

    async fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let mut con = self.pool.get().await?;
        let data: Vec<u8> = con
            .hget(&self.proof_store_key(), id.to_fixed_bytes().as_slice())
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
        self.set_bytes_by_id(id, &data).await?;
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
        let expiration_options =
            HashFieldExpirationOptions::default().set_expiration(SetExpiry::EX(3600));
        con.hset_ex(
            &self.proof_store_key(),
            &expiration_options,
            &[(&id.to_fixed_bytes(), data)],
        )
        .await?;

        Ok(())
    }

    async fn inc_counter_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<u32> {
        let mut con = self.pool.get().await?;
        let new_counter_value: u32 = con
            .hincr(
                &self.proof_store_counters_key(),
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
        self.write_next_jobs_core(jobs, next_jobs).await?;
        let task = JobsTask::new(jobs);
        let next_task = JobsTask::new(next_jobs);
        self.task_graph.lock().await.add_dep(task, next_task);
        Ok(())
    }

    async fn write_multidimensional_jobs(
        &self,
        jobs_levels: &[Vec<QProvingJobDataID>],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()> {
        self.write_multidimensional_jobs_core(jobs_levels, next_jobs)
            .await?;
        let job_levels_count = jobs_levels.len();
        let tasks = jobs_levels.iter().map(|jobs| JobsTask::new(jobs)).collect::<Vec<_>>();
        let next_task = JobsTask::new(next_jobs);
        let mut task_graph = self.task_graph.lock().await;
        for i in 0..job_levels_count {
            let current_next_task = if i == job_levels_count - 1 {
                &next_task
            } else {
                &tasks[i + 1]
            };
            let current_task = &tasks[i];
            task_graph.add_dep(current_task.clone(), current_next_task.clone());
        }
        Ok(())
    }

    async fn write_next_job_tasks(
        &self,
        task: &JobsTask,
        next_task: &JobsTask,
    ) -> anyhow::Result<()> {
        let mut task_graph = self.task_graph.lock().await;
        task_graph.add_dep(next_task.clone(), task.clone());
        Ok(())
    }

    async fn write_multidimensional_job_tasks(
        &self,
        tasks: &[JobsTask],
        next_task: &JobsTask,
    ) -> anyhow::Result<()> {
        let mut task_graph = self.task_graph.lock().await;
        let job_levels_count = tasks.len();
        for i in 0..job_levels_count {
            let current_next_task = if i == job_levels_count - 1 {
                &next_task
            } else {
                &tasks[i + 1]
            };
            let current_task = &tasks[i];
            task_graph.add_dep(current_next_task.clone(), current_task.clone());
        }
        Ok(())
    }
}

#[async_trait]
impl CheckpointDrainQueueEmitterAsyncImm for ProofStoreRedisAsync {
    async fn cdq_push_imm<T: DQSerializable>(&self, item: T) -> anyhow::Result<()> {
        let checkpoint_queue_prefix =
            format!("{}-{}", self.worker_queue_key(), PS_DRAIN_QUEUE_KEY_PREFIX);
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
            format!("{}-{}", self.worker_queue_key(), PS_DRAIN_QUEUE_KEY_PREFIX);
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
            format!("{}-{}", self.worker_queue_key(), PS_DRAIN_QUEUE_KEY_PREFIX);
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
            let job_res: Option<Vec<u8>> = con.lpop(&self.worker_queue_key(), None).await?;
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
            &self.worker_queue_key(),
            jobs.iter().map(|x| x.to_fixed_bytes().to_vec()).collect::<Vec<Vec<u8>>>().as_slice(),
        )
        .await?;

        Ok(())
    }
    async fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        con.lpush(&self.notifications_queue_key(), job.to_fixed_bytes().as_slice())
            .await?;

        Ok(())
    }
}

#[async_trait]
impl WorkerEventTransmitterAsyncImm for ProofStoreRedisAsync {
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        con.lpush(
            &self.worker_queue_key(),
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
            let job_res: Option<Vec<u8>> = con.lpop(&self.notifications_queue_key(), None).await?;
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
                self.checkpoint_history_queue_prefix_key(), metadata.channel_id, metadata.checkpoint_id,
            ),
            bytes.as_slice(),
        )
        .await?;
        con.set(
            format!("{}-{}", self.checkpoint_history_queue_prefix_key(), metadata.channel_id),
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
            .get(format!("{}-{}", self.checkpoint_history_queue_prefix_key(), channel_id))
            .await?;
        match cur_checkpoint_id_opt {
            Some(r) => {
                if r >= start_checkpoint_id {
                    let mut results = Vec::with_capacity((r - start_checkpoint_id + 1) as usize);

                    for i in (start_checkpoint_id..=r) {
                        let result: Vec<u8> = con
                            .get(format!(
                                "{}-{}_{}",
                                self.checkpoint_history_queue_prefix_key(), channel_id, i,
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
                .get(format!("{}-{}", self.checkpoint_history_queue_prefix_key(), channel_id))
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
                self.checkpoint_history_queue_prefix_key(), channel_id, checkpoint_current,
            ))
            .await?;
        Ok(T::from_bytes(&result)?)
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
        let key = format!("{}-{}", self.notifications_queue_key(), item.get_hq_metadata().channel_id);
        conn.rpush(key.as_str(), item.to_bytes()?).await?;
        Ok(())
    }

    async fn consume_item(&self, channel_id: u64) -> anyhow::Result<T> {

        loop {
            let mut conn = self.pool.get().await?;
            let key = format!("{}-{}", self.notifications_queue_key(), channel_id);
            let result: Option<Vec<u8>> = conn.lpop(key.as_str(), None).await?;
            match result {
                Some(result) => {
                    return match T::from_bytes(&result) {
                        Ok(item) => Ok(item),
                        Err(err) => Err(anyhow::anyhow!("Failed to parse item: {:?}", err)),
                    }
                }
                None => {
                    sleep(Duration::from_millis(200)).await;
                }
            }
        }
    }
}