use std::sync::Arc;
use std::time::Duration;
use std::fmt;

use async_trait::async_trait;
use fred::prelude::{HashesInterface, KeysInterface, ListInterface, Pool, FredResult};
use kvq::traits::{KVQPair, KVQSerializable};
use plonky2::plonk::{config::GenericConfig, proof::ProofWithPublicInputs};
use qed_core::job::{
    drain_queue::{
        CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm, DQSerializable,
    },
    history_queue::{
        CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm,
        CheckpointHistoryQueueEmitterSyncImm, HQSerializable,
    },
    id::{QProvingJobDataID, ProvingJobDataId},
    traits::{QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
    worker_queue::{WorkerEventReceiverAsyncImm, WorkerEventTransmitterAsyncImm},
};
use tokio::time::sleep;
use tracing::{debug, info};

pub const PROOF_STORE_KEY_PREFIX_1: &'static str = "PSV1";
pub const PROOF_STORE_COUNTERS_PREFIX_1: &'static str = "proof_counters";

pub const PS_DRAIN_QUEUE_KEY_PREFIX: &'static str = "PSDQV1_";
pub const PS_WORKER_QUEUE_KEY_PREFIX: &'static str = "PSWQV1";
pub const PS_NOTIFICATIONS_QUEUE_KEY_PREFIX: &'static str = "PSNQV1";
pub const PS_HISTORY_QUEUE_KEY_PREFIX: &'static str = "PSHQV1";
pub const REAML_PROOF_KEY: &str = "REALM_PROOF";

#[async_trait]
pub trait SyncProofQueue {
    async fn produce_proof(&self, item: ProvingJobDataId) -> anyhow::Result<()>;
    async fn consume_proof(&self) -> anyhow::Result<ProvingJobDataId>;
}

#[derive(Clone)]
pub struct ProofStoreFred {
    pool: Pool,
    pub worker_queue_id: String,
    notifications_queue_id: String,
    proof_store_key: String,
    proof_store_counters: String,
}

impl fmt::Debug for ProofStoreFred {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProofStoreFred {{ pool: ..., worker_queue_id: {:?}, notifications_queue_id: {:?} }}", 
               self.worker_queue_id, self.notifications_queue_id)
    }
}

/// Wrapper struct that provides only drain queue functionality
#[derive(Clone)]
pub struct DrainQueueFred {
    pool: Pool,
}

impl DrainQueueFred {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

impl fmt::Debug for DrainQueueFred {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DrainQueueFred {{ pool: ... }}")
    }
}

impl ProofStoreFred {
    pub fn new(
        pool: Pool,
        worker_queue_suffix: String,
        notifications_queue_suffix: String,
    ) -> Self {
        Self {
            pool,
            worker_queue_id: format!("{}-{}", PS_WORKER_QUEUE_KEY_PREFIX, worker_queue_suffix),
            notifications_queue_id: format!(
                "{}-{}",
                PS_NOTIFICATIONS_QUEUE_KEY_PREFIX, notifications_queue_suffix
            ),
            proof_store_key: format!("{}", PROOF_STORE_KEY_PREFIX_1),
            proof_store_counters: format!("{}", PROOF_STORE_COUNTERS_PREFIX_1),
        }
    }

    pub fn new2(
        pool: Pool,
        worker_queue_suffix: &str,
        notifications_queue_suffix: &str,
        proof_store_key_suffix: &str,
        proof_store_counters_suffix: &str,
    ) -> Self {
        Self {
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
        }
    }
    pub fn pool(&self) -> &Pool {
        &self.pool
    }
}

#[async_trait]
impl QProofStoreReaderAsync for ProofStoreFred {
    async fn contains_id(&self, id: QProvingJobDataID) -> anyhow::Result<bool> {
        let data = self
            .pool
            .hget::<Vec<u8>, _, &[u8]>(&self.proof_store_key, &id.to_fixed_bytes())
            .await?;
        Ok(data.len() != 0)
    }
    async fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!(?id, "Getting proof by id");
        let data = self
            .pool
            .hget::<Vec<u8>, _, &[u8]>(&self.proof_store_key, &id.to_fixed_bytes())
            .await?;
        tracing::info!(?id, "Got proof by id, data.len = {}", data.len());
        Ok(bincode::deserialize(&data)?)
    }

    async fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let data = self
            .pool
            .hget::<Vec<u8>, _, &[u8]>(&self.proof_store_key, &id.to_fixed_bytes())
            .await?;

        Ok(data)
    }
}

#[async_trait]
impl QProofStoreWriterAsyncImm for ProofStoreFred {
    async fn set_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
    ) -> anyhow::Result<()> {
        tracing::info!(?id, "Setting proof by id");
        let data = bincode::serialize(&proof).unwrap();

        self.pool
            .hsetnx::<(), _, &[u8], Vec<u8>>(&self.proof_store_key, &id.to_fixed_bytes(), data)
            .await
            .unwrap();

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
        self.pool
            .hsetnx::<(), _, &[u8], &[u8]>(&self.proof_store_key, &id.to_fixed_bytes(), data)
            .await?;
        Ok(())
    }

    async fn inc_counter_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<u32> {
        let new_counter_value = self
            .pool
            .hincrby::<u32, _, &[u8]>(&self.proof_store_counters, &id.to_fixed_bytes(), 1)
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
impl CheckpointDrainQueueEmitterAsyncImm for ProofStoreFred {
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
        self.pool.lpush::<(), String, &[u8]>(key, &bytes).await?;

        Ok(())
    }
}

#[async_trait]
impl CheckpointDrainQueueConsumerAsyncImm for ProofStoreFred {
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
        let members: Vec<Vec<u8>> = self
            .pool
            .lrange::<Vec<Vec<u8>>, String>(key.clone(), 0, -1)
            .await?;
        self.pool.del::<(), String>(key).await?;

        members
            .into_iter()
            .rev()
            .map(|x| T::from_bytes(&x))
            .collect()
    }
}

#[async_trait]
impl WorkerEventReceiverAsyncImm for ProofStoreFred {
    async fn wait_for_next_job_imm(&self) -> anyhow::Result<QProvingJobDataID> {
        loop {
            let job_res = self
                .pool
                .lpop::<[u8; 24], _>(&self.worker_queue_id, None)
                .await;
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
        self.pool
            .lpush::<(), _, Vec<Vec<u8>>>(
                &self.worker_queue_id,
                jobs.iter().map(|x| x.to_fixed_bytes().to_vec()).collect(),
            )
            .await?;

        Ok(())
    }
    async fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()> {
        self.pool
            .lpush::<(), _, &[u8]>(&self.notifications_queue_id, &job.to_fixed_bytes())
            .await?;

        Ok(())
    }
}

#[async_trait]
impl WorkerEventTransmitterAsyncImm for ProofStoreFred {
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        self.pool
            .lpush::<(), _, Vec<Vec<u8>>>(
                &self.worker_queue_id,
                jobs.iter().map(|x| x.to_fixed_bytes().to_vec()).collect(),
            )
            .await?;

        Ok(())
    }
    async fn wait_for_block_proving_jobs_imm(
        &self,
        _checkpoint_id: u64,
    ) -> anyhow::Result<QProvingJobDataID> {
        loop {
            let job_res = self
                .pool
                .lpop::<Vec<u8>, _>(&self.notifications_queue_id, None)
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
}

#[async_trait]
impl CheckpointHistoryQueueEmitterAsyncImm for ProofStoreFred {
    async fn chq_push_imm<T: HQSerializable>(&self, item: T) -> anyhow::Result<()> {
        let metadata = item.get_hq_metadata();
        let bytes = item.to_bytes()?;
        self.pool
            .set::<(), String, &[u8]>(
                format!(
                    "{}-{}_{}",
                    PS_HISTORY_QUEUE_KEY_PREFIX, metadata.channel_id, metadata.checkpoint_id,
                ),
                &bytes,
                None,
                None,
                false,
            )
            .await?;
        self.pool
            .set::<(), String, u64>(
                format!("{}-{}", PS_HISTORY_QUEUE_KEY_PREFIX, metadata.channel_id,),
                metadata.checkpoint_id,
                None,
                None,
                false,
            )
            .await?;
        Ok(())
    }
}

#[async_trait]
impl CheckpointHistoryQueueConsumerAsyncImm for ProofStoreFred {
    async fn chq_listen_from_imm<T: HQSerializable>(
        &self,
        channel_id: u64,
        start_checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let cur_checkpoint_id = self
            .pool
            .get::<Option<u64>, String>(format!("{}-{}", PS_HISTORY_QUEUE_KEY_PREFIX, channel_id,))
            .await?;
        match cur_checkpoint_id {
            Some(r) => {
                if r >= start_checkpoint_id {
                    let mut results = Vec::with_capacity((r - start_checkpoint_id + 1) as usize);

                    for i in (start_checkpoint_id..=r) {
                        let result = self
                            .pool
                            .get::<Vec<u8>, String>(format!(
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
        let cur_checkpoint_id = self
            .pool
            .get::<Option<u64>, String>(format!("{}-{}", PS_HISTORY_QUEUE_KEY_PREFIX, channel_id,))
            .await?;
        let mut checkpoint_current: i64 = match cur_checkpoint_id {
            Some(x) => x as i64,
            None => -1,
        };

        let start_i64 = start_checkpoint_id as i64;
        while checkpoint_current < start_i64 {
            sleep(Duration::from_millis(100)).await;
            let cur_checkpoint_id = self
                .pool
                .get::<Option<u64>, String>(format!(
                    "{}-{}",
                    PS_HISTORY_QUEUE_KEY_PREFIX, channel_id,
                ))
                .await?;
            checkpoint_current = match cur_checkpoint_id {
                Some(x) => x as i64,
                None => -1,
            };
        }
        let result = self
            .pool
            .get::<Vec<u8>, String>(format!(
                "{}-{}_{}",
                PS_HISTORY_QUEUE_KEY_PREFIX, channel_id, checkpoint_current,
            ))
            .await?;
        Ok(T::from_bytes(&result)?)
    }
    
    async fn is_empty(&self) -> anyhow::Result<bool> {
        // Check if REALM_CHECKPOINT queue is empty
        let realm_checkpoint_key = format!("{}-REALM_CHECKPOINT", self.worker_queue_id);
        let length: usize = self.pool.llen(&realm_checkpoint_key).await?;
        Ok(length == 0)
    }
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

// DrainQueueFred trait implementations - for backward compatibility
#[async_trait]
impl CheckpointDrainQueueEmitterAsyncImm for DrainQueueFred {
    async fn cdq_push_imm<T: DQSerializable>(&self, item: T) -> anyhow::Result<()> {
        let metadata = item.get_dq_metadata();
        let bytes = item.to_bytes()?;
        self.pool
            .lpush::<(), String, &[u8]>(
                format!("CDQ_2_{}_{}", metadata.channel_id, metadata.checkpoint_id),
                &bytes,
            )
            .await?;
        Ok(())
    }
}

#[async_trait]
impl CheckpointDrainQueueConsumerAsyncImm for DrainQueueFred {
    async fn cdq_drain_imm<T: DQSerializable>(
        &self,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let key = format!("CDQ_2_{}_{}", channel_id, checkpoint_id);
        let members: Vec<Vec<u8>> = self
            .pool
            .lrange::<Vec<Vec<u8>>, String>(key.clone(), 0, -1)
            .await?;
        self.pool.del::<(), String>(key).await?;

        members
            .into_iter()
            .rev()
            .map(|x| T::from_bytes(&x))
            .collect()
    }
}