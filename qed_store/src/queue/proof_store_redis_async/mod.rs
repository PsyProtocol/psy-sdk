use std::time::Duration;

use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;

use super::proof_store_fred::{
    PROOF_STORE_COUNTERS_PREFIX_1, PROOF_STORE_KEY_PREFIX_1, PS_DRAIN_QUEUE_KEY_PREFIX,
    PS_HISTORY_QUEUE_KEY_PREFIX, PS_NOTIFICATIONS_QUEUE_KEY_PREFIX, PS_WORKER_QUEUE_KEY_PREFIX,
};
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
use tokio::time::sleep;

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
            .hget(&self.proof_store_key, &id.to_fixed_bytes())
            .await?;
        Ok(data.len() != 0)
    }

    async fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!(?id, "Getting proof by id");
        let mut con = self.pool.get().await?;
        let data: Vec<u8> = con
            .hget(&self.proof_store_key, &id.to_fixed_bytes())
            .await?;
        tracing::info!(?id, "Got proof by id, data.len = {}", data.len());
        Ok(bincode::deserialize(&data)?)
    }

    async fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let mut con = self.pool.get().await?;
        let data = con
            .hget(&self.proof_store_key, &id.to_fixed_bytes())
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
        con.hset_nx(&self.proof_store_key, &id.to_fixed_bytes(), data)
            .await?;
        Ok(())
    }
    async fn set_bytes_by_id_batch(
        &self,
        kv_pairs: &[KVQPair<QProvingJobDataID, Vec<u8>>],
    ) -> anyhow::Result<()> {
        //todo: implemeent more efficient?
        self.set_bytes_by_id_batch_core(kv_pairs).await
    }
    async fn set_bytes_by_id(&self, id: QProvingJobDataID, data: &[u8]) -> anyhow::Result<()> {
        // tracing::info!(?id, "Setting bytes by id, data.len = {}", data.len());
        let mut con = self.pool.get().await?;
        con.hset_nx(&self.proof_store_key, &id.to_fixed_bytes(), data)
            .await?;
        Ok(())
    }

    async fn inc_counter_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<u32> {
        let mut con = self.pool.get().await?;
        let new_counter_value = con
            .hincr(&self.proof_store_counters, &id.to_fixed_bytes(), 1)
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
            "{}-{}_{}",
            checkpoint_queue_prefix, metadata.channel_id, metadata.checkpoint_id
        );
        // tracing::debug!("Pushing job id to queue: {:?}", key);
        let mut con = self.pool.get().await?;
        con.lpush(&key, &bytes).await?;

        Ok(())
    }
}

#[async_trait]
impl CheckpointDrainQueueConsumerAsyncImm for ProofStoreRedisAsync {
    async fn cdq_drain_imm<T: DQSerializable>(
        &self,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let checkpoint_queue_prefix =
            format!("{}-{}", self.worker_queue_id, PS_DRAIN_QUEUE_KEY_PREFIX);
        let key = format!(
            "{}-{}_{}",
            checkpoint_queue_prefix, channel_id, checkpoint_id
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
}

#[async_trait]
impl WorkerEventReceiverAsyncImm for ProofStoreRedisAsync {
    async fn wait_for_next_job_imm(&self) -> anyhow::Result<QProvingJobDataID> {
        loop {
            let mut con = self.pool.get().await?;
            let job_res = con.lpop::<_, [u8; 24]>(&self.worker_queue_id, None).await;
            match job_res {
                Ok(g) => {
                    return Ok(QProvingJobDataID::try_from_byte_vec(&g)?);
                }
                Err(e) => {} // println!("error: {:?}", e)
                             ,
            };
            sleep(Duration::from_millis(100)).await;
        }
    }

    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        con.lpush(
            &self.worker_queue_id,
            jobs.iter()
                .map(|x| x.to_fixed_bytes().to_vec())
                .collect::<Vec<Vec<u8>>>(),
        )
        .await?;
        Ok(())
    }
    async fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        con.lpush(&self.notifications_queue_id, &job.to_fixed_bytes())
            .await?;

        Ok(())
    }
    /*
    fn wait_for_next_job_mut(&mut self) -> anyhow::Result<QProvingJobDataID> {
        loop {
            let job = self.job_queue.pop_one(Q_JOB)?;
            if job.is_some() {
                return Ok(serde_json::from_slice(&job.unwrap())?)
            }else{
                std::thread::sleep(Duration::from_millis(250));
                continue;
            }
        }
    }

    fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        for job in jobs {
            self.job_queue.dispatch(Q_JOB, job.clone())?;
        }
        Ok(())
    }

    fn notify_core_goal_completed_mut(&mut self, _job: QProvingJobDataID) -> anyhow::Result<()> {
        self.job_queue.dispatch(Q_NOTIFICATIONS, QueueNotification::CoreJobCompleted)?;
        Ok(())
    }

    fn record_job_bench_mut(&mut self, job: QProvingJobDataID, duration: u64) -> anyhow::Result<()> {
        if self.benckmarks_enabled {
            self.benchmarks.push(QWorkerJobBenchmark {
                job_id: job.to_fixed_bytes(),
                duration,
            });
        }
        Ok(())
    }*/
}

#[async_trait]
impl WorkerEventTransmitterAsyncImm for ProofStoreRedisAsync {
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        con.lpush(
            &self.worker_queue_id,
            jobs.iter()
                .map(|x| x.to_fixed_bytes().to_vec())
                .collect::<Vec<Vec<u8>>>(),
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
            let job_res = con.lpop::<_, Vec<u8>>(&self.notifications_queue_id, None)
                .await;
            match job_res {
                Ok(g) => {
                    if g.len() == 24 {
                        match QProvingJobDataID::try_from_byte_vec(&g) {
                            Ok(job) => {
                                if job.is_notify_complete() {
                                    return Ok(job)
                                }
                            },
                            Err(e1) => eprintln!("error deserializing job id in wait_for_block_proving_jobs_imm: {:?}", e1),
                        }
                    }
                }
                Err(e2) => eprintln!(
                    "error deserializing job id in wait_for_block_proving_jobs_imm: {:?}",
                    e2
                ),
            };
            sleep(Duration::from_millis(500)).await;
        }
    }
    /*
    fn enqueue_jobs_mut(&mut self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        for job in jobs {
            self.job_queue.dispatch(Q_JOB, job.clone())?;
        }
        Ok(())
    }

    fn wait_for_block_proving_jobs_mut(&mut self, _checkpoint_id: u64) -> anyhow::Result<bool> {
        loop {
            match self
                .job_queue
                .pop_one(Q_NOTIFICATIONS)?
                .map(|v| serde_json::from_slice::<QueueNotification>(&v))
            {
                Some(Ok(QueueNotification::CoreJobCompleted)) => return Ok::<_, anyhow::Error>(true),
                Some(Err(_)) | None => {
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }
    }*/
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
            bytes,
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
        let cur_checkpoint_id = con
            .get(format!("{}-{}", PS_HISTORY_QUEUE_KEY_PREFIX, channel_id,))
            .await?;
        match cur_checkpoint_id {
            Some(r) => {
                if r >= start_checkpoint_id {
                    let mut results = Vec::with_capacity((r - start_checkpoint_id + 1) as usize);

                    for i in (start_checkpoint_id..=r) {
                        let result = con
                            .get::<_, Vec<u8>>(format!(
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
        let mut con = self.pool.get().await?;
        let cur_checkpoint_id: Option<i64> = con
            .get(format!("{}-{}", PS_HISTORY_QUEUE_KEY_PREFIX, channel_id,))
            .await?;
        let mut checkpoint_current: i64 = match cur_checkpoint_id {
            Some(x) => x as i64,
            None => -1,
        };

        let start_i64 = start_checkpoint_id as i64;
        while checkpoint_current < start_i64 {
            sleep(Duration::from_millis(100)).await;
            let cur_checkpoint_id: Option<i64> = con
                .get(format!("{}-{}", PS_HISTORY_QUEUE_KEY_PREFIX, channel_id,))
                .await?;
            checkpoint_current = match cur_checkpoint_id {
                Some(x) => x as i64,
                None => -1,
            };
        }
        let result: Vec<u8> = con
            .get(format!(
                "{}-{}_{}",
                PS_HISTORY_QUEUE_KEY_PREFIX, channel_id, checkpoint_current,
            ))
            .await?;
        Ok(T::from_bytes(&result)?)
    }
}
