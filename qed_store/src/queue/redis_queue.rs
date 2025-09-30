use std::{sync::Arc, time::Duration};
use std::time::Instant;
use auto_impl::auto_impl;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use redis::{AsyncCommands, HashFieldExpirationOptions, SetExpiry};
use qed_core::data::qhashout::QHashOut;
use serde::{Deserialize, Serialize};

use async_trait::async_trait;
use kvq::traits::{KVQPair, KVQSerializable};
use plonky2::{hash::hash_types::RichField, plonk::{config::GenericConfig, proof::ProofWithPublicInputs}};
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
use tracing::error;
// Re-use constants from fred_queue
use crate::queue::{PROOF_STORE_COUNTERS_PREFIX_1, PROOF_STORE_KEY_PREFIX_1, PS_DRAIN_QUEUE_KEY_PREFIX, PS_HISTORY_QUEUE_KEY_PREFIX, PS_NOTIFICATIONS_QUEUE_KEY_PREFIX, PS_WORKER_QUEUE_KEY_PREFIX};

pub const REALM_PENDING_USER_QUEUE_KEY_PREFIX: &'static str = "RMPUQ";
pub const MAX_CHECKPOINT_COUNT: usize = 256;

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

    // realm pending user key prefix REALM_PENDING_USER_QUEUE_KEY_PREFIX
    fn realm_pending_user_key(&self) -> String;

    // checkpoint management keys
    fn checkpoint_list_key(&self) -> String;
    fn checkpoint_proofs_key(&self, checkpoint_id: u64) -> String;

    // public inputs key
    fn public_inputs_key(&self) -> String;
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

    fn realm_pending_user_key(&self) -> String {
        format!("{}-{}", REALM_PENDING_USER_QUEUE_KEY_PREFIX, self.biz_key())
    }

    fn checkpoint_list_key(&self) -> String {
        format!("checkpoint_list-{}", self.biz_key())
    }

    fn checkpoint_proofs_key(&self, checkpoint_id: u64) -> String {
        format!("checkpoint_proofs:{}-{}", checkpoint_id, self.biz_key())
    }

    fn public_inputs_key(&self) -> String {
        format!("public_inputs-{}", self.biz_key())
    }
}

#[derive(Debug, Clone)]
pub struct ProofStoreRedisAsync {
    pool: Pool<RedisConnectionManager>,
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
            biz_key: biz_key,
        })
    }
    pub fn pool(&self) -> &Pool<RedisConnectionManager> {
        &self.pool
    }

    async fn add_checkpoint(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        let checkpoint_list_key = self.checkpoint_list_key();
        let _: () = con.sadd(&checkpoint_list_key, checkpoint_id).await?;
        Ok(())
    }

}

#[async_trait]
impl QProofStoreReaderAsync for ProofStoreRedisAsync {
    async fn contains_id(&self, id: QProvingJobDataID) -> anyhow::Result<bool> {
        let checkpoint_id = id.goal_id;
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);

        let mut con = self.pool.get().await?;
        let data: Vec<u8> = con
            .hget(&checkpoint_proofs_key, id.to_fixed_bytes().as_slice())
            .await?;
        Ok(!data.is_empty())
    }
    async fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let checkpoint_id = id.goal_id;
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);

        let mut con = self.pool.get().await?;
        let data: Vec<u8> = con
            .hget(&checkpoint_proofs_key, id.to_fixed_bytes().as_slice())
            .await?;
        Ok(bincode::deserialize(&data)?)
    }

    async fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let checkpoint_id = id.goal_id;
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);

        let mut con = self.pool.get().await?;
        let data: Vec<u8> = con
            .hget(&checkpoint_proofs_key, id.to_fixed_bytes().as_slice())
            .await?;
        Ok(data)
    }

    async fn get_public_input_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<Vec<C::F>> {
        let mut con = self.pool.get().await?;
        let public_inputs_key = self.public_inputs_key();
        let data: Vec<u8> = con
            .hget(&public_inputs_key, id.to_fixed_bytes().as_slice())
            .await?;
        Ok(bincode::deserialize(&data)?)
    }
}

#[async_trait]
impl QProofStoreWriterAsyncImm for ProofStoreRedisAsync {
    async fn set_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
    ) -> anyhow::Result<()> {
        let data = bincode::serialize(&proof)?;
        let checkpoint_id = id.goal_id;

        self.add_checkpoint(checkpoint_id).await?;

        // Store proof to corresponding checkpoint's hashmap
        let mut con = self.pool.get().await?;
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);
        let _: bool = con
            .hset_nx(
                &checkpoint_proofs_key,
                id.to_fixed_bytes().as_slice(),
                data.as_slice(),
            )
            .await?;

        // Store public_inputs separately to a permanent queue
        let public_inputs_data = bincode::serialize(&proof.public_inputs)?;
        let public_inputs_key = self.public_inputs_key();
        let _: bool = con
            .hset_nx(
                &public_inputs_key,
                id.to_fixed_bytes().as_slice(),
                public_inputs_data.as_slice(),
            )
            .await?;

        Ok(())
    }
    async fn set_bytes_by_id_batch(
        &self,
        kv_pairs: &[KVQPair<QProvingJobDataID, Vec<u8>>],
    ) -> anyhow::Result<()> {
        self.set_bytes_by_id_batch_core(kv_pairs).await
    }
    async fn set_bytes_by_id(&self, id: QProvingJobDataID, data: &[u8]) -> anyhow::Result<()> {
        let checkpoint_id = id.goal_id;

        // Manage checkpoint limit
        self.add_checkpoint(checkpoint_id).await?;

        // Store data to corresponding checkpoint's hashmap
        let mut con = self.pool.get().await?;
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);
        let _: bool = con
            .hset_nx(
                &checkpoint_proofs_key,
                id.to_fixed_bytes().as_slice(),
                data,
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
        Ok(())
    }

    async fn write_multidimensional_jobs(
        &self,
        jobs_levels: &[Vec<QProvingJobDataID>],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()> {
        self.write_multidimensional_jobs_core(jobs_levels, next_jobs)
            .await?;
        Ok(())
    }

    async fn cleanup_old_proofs(&self, current_height: u64, keep_blocks: u64) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        let checkpoint_list_key = self.checkpoint_list_key();

        let threshold = current_height.saturating_sub(keep_blocks);
        let all_checkpoints: Vec<u64> = con.smembers(&checkpoint_list_key).await?;

        let to_remove: Vec<_> = all_checkpoints
            .into_iter()
            .filter(|&id| id < threshold)
            .collect();

        for checkpoint_id in &to_remove {
            let key = self.checkpoint_proofs_key(*checkpoint_id);
            let _: () = con.del(&key).await?;
        }

        if !to_remove.is_empty() {
            let _: () = con.srem(&checkpoint_list_key, &to_remove).await?;
            tracing::info!("Cleaned up {} old checkpoints < {}", to_remove.len(), threshold);
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
        con.rpush(key, bytes).await?;

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
            .map(|x| T::from_bytes(&x))
            .collect()
    }
}

#[async_trait]
pub trait CheckpointDrainQueueConsumerAsyncImmWithPosition: CheckpointDrainQueueConsumerAsyncImm {
    async fn peek_with_position<T: DQSerializable>(
        &self,
        count: Option<isize>,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<(Vec<T>, QueueOffsetState)>;

    async fn get_last_peek_offset(
        &self,
        channel_id: u64,
    ) -> anyhow::Result<Option<QueueOffsetState>>;

    async fn commit_offset(&self, state: &QueueOffsetState) -> anyhow::Result<()>;
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
        con.rpush(
            &self.worker_queue_key(),
            jobs.iter().map(|x| x.to_fixed_bytes().to_vec()).collect::<Vec<Vec<u8>>>().as_slice(),
        )
        .await?;

        Ok(())
    }
    async fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        con.rpush(&self.notifications_queue_key(), job.to_fixed_bytes().as_slice())
            .await?;

        Ok(())
    }
}

#[async_trait]
impl WorkerEventTransmitterAsyncImm for ProofStoreRedisAsync {
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        let mut con = self.pool.get().await?;
        con.rpush(
            &self.worker_queue_key(),
            jobs.iter().map(|x| x.to_fixed_bytes().to_vec()).collect::<Vec<Vec<u8>>>().as_slice(),
        )
        .await?;

        Ok(())
    }
    async fn wait_for_block_proving_jobs_imm(
        &self,
        _checkpoint_id: u64,
        timeout: Option<Duration>,
    ) -> anyhow::Result<QProvingJobDataID> {
        let now = Instant::now();
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
                                } else {
                                    error!("job is not notify complete, checkpoint_id = {}", _checkpoint_id);
                                }
                            }
                            Err(e1) => error!(
                                "error deserializing job id in wait_for_block_proving_jobs_imm: {:?}, checkpoint_id = {}",
                                e1, _checkpoint_id,
                            ),
                        }
                    } else {
                        error!("g.len() != 24, g.len() = {}, checkpoint_id = {}", g.len(), _checkpoint_id);
                    }
                }
                None => {}
            };
            if let Some(timeout) = timeout{
                if now.elapsed() > timeout {
                    return Err(anyhow::anyhow!("timeout waiting for job"))
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    async fn wait_for_job_proof<C: GenericConfig<D> + 'static, const D: usize>(
        &self,
        job_id: QProvingJobDataID,
        timeout: Option<Duration>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>
    where
        C::Hasher: plonky2::plonk::config::AlgebraicHasher<C::F>
    {
        let now = Instant::now();
        loop {
            match self.get_proof_by_id::<C, D>(job_id.get_output_id()).await {
                Ok(proof) => return Ok(proof),
                Err(_) => {
                    sleep(Duration::from_millis(100)).await;
                }
            }
            if let Some(timeout) = timeout{
                if now.elapsed() > timeout {
                    return Err(anyhow::anyhow!("timeout waiting for job"))
                }
            }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueOffsetState {
    pub start_position: i64,  // Redis list position (0-based)
    pub end_position: i64,    // End position (inclusive)
    pub checkpoint_id: u64,
    pub channel_id: u64,
    pub consumed_count: usize,
}

#[async_trait]
pub trait QPendingUserStoreAsyncImm: Send + Sync {
    async fn push_pending_users<F: RichField>(
        &self,
        pending_users: &[MerkleProofCore<QHashOut<F>>],
    ) -> anyhow::Result<()>;
    async fn pop_pending_users<F: RichField>(
        &self,
        count: usize,
    ) -> anyhow::Result<Vec<MerkleProofCore<QHashOut<F>>>>;
    async fn get_pending_users_count(&self) -> anyhow::Result<usize>;

    // consume_users_with_position for position-based consumption
    async fn peek_with_position<F: RichField>(
        &self,
        count: usize,
        checkpoint_id: u64,
    ) -> anyhow::Result<(Vec<MerkleProofCore<QHashOut<F>>>, QueueOffsetState)>;

    async fn commit_offset(&self, state: &QueueOffsetState) -> anyhow::Result<()>;
    async fn get_last_peek_offset(&self) -> anyhow::Result<Option<QueueOffsetState>>;
}

#[async_trait]
impl QPendingUserStoreAsyncImm for ProofStoreRedisAsync {
    async fn push_pending_users<F: RichField>(
        &self,
        pending_users: &[MerkleProofCore<QHashOut<F>>],
    ) -> anyhow::Result<()> {
        if pending_users.is_empty() {
            return Ok(());
        }

        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");

        for user in pending_users.iter() {
            let user_bytes = bincode::serialize(user).map_err(|e| anyhow::anyhow!(e))?;
            conn.rpush(&key, user_bytes).await?;
        }

        Ok(())
    }

    async fn pop_pending_users<F: RichField>(
        &self,
        count: usize,
    ) -> anyhow::Result<Vec<MerkleProofCore<QHashOut<F>>>> {
        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");

        let mut users = Vec::new();
        for _ in 0..count {
            let user_bytes: Option<Vec<u8>> = conn.lpop(&key, None).await?;
            if let Some(bytes) = user_bytes {
                let user = bincode::deserialize(&bytes).map_err(|e| anyhow::anyhow!(e))?;
                users.push(user);
            } else {
                break; // No more users in queue
            }
        }

        Ok(users)
    }

    async fn get_pending_users_count(&self) -> anyhow::Result<usize> {
        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");
        let length: usize = conn.llen(key).await?;
        Ok(length)
    }

    async fn peek_with_position<F: RichField>(
        &self,
        count: usize,
        checkpoint_id: u64,
    ) -> anyhow::Result<(Vec<MerkleProofCore<QHashOut<F>>>, QueueOffsetState)> {
        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");

        // Read items without removing them
        let start_position = 0i64;
        let items: Vec<Vec<u8>> = conn.lrange(&key, start_position as isize, (count - 1) as isize).await?;

        // Deserialize items
        let mut users = Vec::with_capacity(items.len());
        for item_bytes in items {
            let user = bincode::deserialize(&item_bytes).map_err(|e| anyhow::anyhow!(e))?;
            users.push(user);
        }
        let end_position = if users.is_empty() { -1 } else { users.len() as i64 - 1 };
        let state = QueueOffsetState {
            start_position,
            end_position,
            checkpoint_id,
            channel_id: 0,
            consumed_count: users.len(),
        };

        // Save consumption state for potential recovery
        let state_key = format!("{}-{}", self.realm_pending_user_key(), "CONSUMPTION_STATE");
        let state_data = bincode::serialize(&state).map_err(|e| anyhow::anyhow!(e))?;
        conn.set_ex(&state_key, state_data, 3600).await?; // 1 hour TTL

        tracing::debug!("Consumed {} users from positions {}-{} for checkpoint {}",
              users.len(), start_position, end_position, checkpoint_id);

        Ok((users, state))
    }

    async fn commit_offset(&self, state: &QueueOffsetState) -> anyhow::Result<()> {
        if state.consumed_count == 0 {
            return Ok(());
        }

        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");

        // Remove consumed items from the queue using LTRIM
        conn.ltrim(&key, (state.end_position + 1) as isize, -1).await?;

        // Clear consumption state
        let state_key = format!("{}-{}", self.realm_pending_user_key(), "CONSUMPTION_STATE");
        conn.del(&state_key).await?;

        tracing::debug!("Committed consumption of {} users for checkpoint {}",
              state.consumed_count, state.checkpoint_id);

        Ok(())
    }

    async fn get_last_peek_offset(&self) -> anyhow::Result<Option<QueueOffsetState>> {
        let mut conn = self.pool().get().await?;
        let state_key = format!("{}-{}", self.realm_pending_user_key(), "CONSUMPTION_STATE");

        let state_data: Option<Vec<u8>> = conn.get(&state_key).await?;
        if let Some(data) = state_data {
            let state: QueueOffsetState = bincode::deserialize(&data).map_err(|e| anyhow::anyhow!(e))?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }
}


#[async_trait]
impl CheckpointDrainQueueConsumerAsyncImmWithPosition for ProofStoreRedisAsync {
    async fn peek_with_position<T: DQSerializable>(
        &self,
        count: Option<isize>,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<(Vec<T>, QueueOffsetState)> {
        let checkpoint_queue_prefix =
            format!("{}-{}", self.worker_queue_key(), PS_DRAIN_QUEUE_KEY_PREFIX);
        let key = format!(
            "{}-{}",
            checkpoint_queue_prefix, channel_id
        );

        let mut con = self.pool.get().await?;
        let members: Vec<Vec<u8>> = con.lrange(key.clone(), 0, (count.unwrap_or_default() - 1) as isize).await?;
        // Deserialize items
        let items: Vec<T> = members
            .into_iter()
            .map(|x| T::from_bytes(&x))
            .collect::<anyhow::Result<Vec<T>>>()?;
        let end_position: i64 = if items.is_empty() {
            -1
        } else {
            items.len() as i64 - 1
        };
        let state = QueueOffsetState {
            start_position: 0i64,
            end_position,
            checkpoint_id,
            channel_id,
            consumed_count: items.len(),
        };
        // Save consumption state for potential recovery
        let state_key = format!("{}-{}-{}", self.worker_queue_key(), "DRAIN_CONSUMPTION_STATE", channel_id);
        let state_data = bincode::serialize(&state).map_err(|e| anyhow::anyhow!(e))?;
        con.set_ex(&state_key, state_data, 3600).await?; // 1 hour TTL

        tracing::debug!("Consumed redis {} items from drain queue {} for checkpoint {}",
              items.len(), channel_id, checkpoint_id);

        Ok((items, state))
    }

    async fn get_last_peek_offset(
        &self,
        channel_id: u64,
    ) -> anyhow::Result<Option<QueueOffsetState>> {
        let state_key = format!("{}-{}-{}", self.worker_queue_key(), "DRAIN_CONSUMPTION_STATE", channel_id);
        let mut con = self.pool.get().await?;
        let state_data: Option<Vec<u8>> = con.get(&state_key).await?;
        if let Some(data) = state_data {
            let state: QueueOffsetState = bincode::deserialize(&data).map_err(|e| anyhow::anyhow!(e))?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    async fn commit_offset(&self, state: &QueueOffsetState) -> anyhow::Result<()> {
        if state.consumed_count == 0 {
            return Ok(());
        }
        let mut con = self.pool.get().await?;
        let checkpoint_queue_prefix =
            format!("{}-{}", self.worker_queue_key(), PS_DRAIN_QUEUE_KEY_PREFIX);
        let key = format!("{}-{}", checkpoint_queue_prefix, state.channel_id);
        // Remove consumed items
        con.ltrim(&key, (state.end_position + 1) as isize, -1).await?;
        // Clear consumption state
        let state_key = format!("{}-{}-{}", self.worker_queue_key(), "DRAIN_CONSUMPTION_STATE", state.channel_id);
        con.del(&state_key).await?;
        tracing::debug!("Consumed redis {} items for checkpoint {}",
              state.consumed_count, state.checkpoint_id);

        Ok(())
    }
}


#[async_trait]
#[auto_impl(Box, Arc)]
pub trait PendingCheckPointAsync<T: KVQSerializable + Send + Sync> {
    async fn set_pending_checkpoint(
        &self,
        item: Option<T>,
    ) -> anyhow::Result<()> where T: 'async_trait;
    async fn get_pending_checkpoint(&self) -> anyhow::Result<Option<T>>;
}

#[async_trait]
impl<T: KVQSerializable + Send+ Sync> PendingCheckPointAsync<T> for ProofStoreRedisAsync {
    async fn set_pending_checkpoint(
        &self,
        item: Option<T>,
    ) -> anyhow::Result<()> where T: 'async_trait {
        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.checkpoint_list_key(), "PENDING_CHECKPOINT");
        if let Some(item) =  item {
            let bytes = item.to_bytes()?;
            conn.set(key, bytes).await?;
        } else {
            conn.del(&key).await?;
        }
        Ok(())
    }
    async fn get_pending_checkpoint(&self) -> anyhow::Result<Option<T>> {
        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.checkpoint_list_key(), "PENDING_CHECKPOINT");
        let bytes: Option<Vec<u8>> = conn.get(&key).await?;
        match bytes {
            Some(bytes) => Ok(Some(T::from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }
}