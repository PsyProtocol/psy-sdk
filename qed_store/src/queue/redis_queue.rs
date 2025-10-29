use std::{sync::Arc, time::Duration};
use std::time::Instant;
use auto_impl::auto_impl;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use redis::{AsyncCommands, HashFieldExpirationOptions, SetExpiry, Value};
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
use crate::queue::{
    resilient_redis::ResilientRedisConnection,
    {PROOF_STORE_COUNTERS_PREFIX_1, PROOF_STORE_KEY_PREFIX_1, PS_DRAIN_QUEUE_KEY_PREFIX, PS_HISTORY_QUEUE_KEY_PREFIX, PS_NOTIFICATIONS_QUEUE_KEY_PREFIX, PS_WORKER_QUEUE_KEY_PREFIX}
};

pub const REALM_PENDING_USER_QUEUE_KEY_PREFIX: &'static str = "RMPUQ";
pub const MAX_CHECKPOINT_COUNT: usize = 256;

#[auto_impl(&, Box, Arc)]
pub trait BizKey {
    fn biz_key(&self) -> String;
}


pub trait QueuePrefixKey {
    fn worker_queue_key(&self) -> String;


    fn drain_queue_key(&self, channel_id: u64) -> String {
        format!("{}-{}-{}", self.worker_queue_key(), PS_DRAIN_QUEUE_KEY_PREFIX,channel_id)
    }
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
    fn id_key(&self, channel_id: u64) -> String;
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

    fn id_key(&self, channel_id: u64) -> String {
        format!("ids:{}-{}", channel_id, self.biz_key())
    }


    fn public_inputs_key(&self) -> String {
        format!("public_inputs-{}", self.biz_key())
    }
}

#[derive(Debug, Clone)]
pub struct ProofStoreRedisAsync {
    pub(crate) redis: ResilientRedisConnection,
    pub(crate) redis_blocking: ResilientRedisConnection,
    pub(crate) biz_key: String,
}

impl BizKey for ProofStoreRedisAsync {
    fn biz_key(&self) -> String {
        self.biz_key.clone()
    }
}

impl ProofStoreRedisAsync {
    pub async fn new(redis_url: &str, biz_key: String) -> anyhow::Result<Self> {
        let redis = ResilientRedisConnection::new(redis_url).await?;
        let redis_blocking = ResilientRedisConnection::new(redis_url).await?;
        Ok(Self {
            redis,
            redis_blocking,
            biz_key,
        })
    }

    async fn add_checkpoint(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        let checkpoint_list_key = self.checkpoint_list_key();
        self.redis.sadd(checkpoint_list_key, checkpoint_id).await?;
        Ok(())
    }
}

#[async_trait]
impl QProofStoreReaderAsync for ProofStoreRedisAsync {

    async fn contains_item(&self, channel_id: u64, id: u64) -> anyhow::Result<bool> {
        let server_time = self.redis.server_time().await?;
        let id_key = self.id_key(channel_id);
        let exists:Option<i64> = self.redis.zscore(id_key, id).await?;
        if let Some(score) = exists {
            if score >= server_time[0] - 1800 { // 30 minutes
                return Ok(true);
            }
        }
        Ok(false)
    }
    async fn contains_id(&self, id: QProvingJobDataID) -> anyhow::Result<bool> {
        let checkpoint_id = id.goal_id;
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);

        self.redis.hexists(checkpoint_proofs_key, id.to_fixed_bytes().to_vec()).await
    }
    async fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let bytes = self.get_bytes_by_id(id.get_output_id()).await?;
        let proof = bincode::deserialize(&bytes)?;
        Ok(proof)
    }

    async fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let checkpoint_id = id.goal_id;
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);

        self.redis.hget(checkpoint_proofs_key, id.to_fixed_bytes().to_vec()).await
    }

    async fn get_public_input_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<Vec<C::F>> {
        let public_inputs_key = self.public_inputs_key();
        let data: Vec<u8> = self.redis.hget(public_inputs_key, id.to_fixed_bytes().to_vec()).await?;
        let public_inputs = bincode::deserialize(&data)?;
        Ok(public_inputs)
    }
}

#[async_trait]
impl QProofStoreWriterAsyncImm for ProofStoreRedisAsync {
    async fn set_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
    ) -> anyhow::Result<()> {
        let checkpoint_id = id.goal_id;
        let checkpoint_list_key = self.checkpoint_list_key();
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);
        let public_inputs_key = self.public_inputs_key();

        let proof_bytes = bincode::serialize(proof)?;
        let public_inputs_data = bincode::serialize(&proof.public_inputs)?;

        self.redis.cmd_builder()
            .sadd(checkpoint_list_key, checkpoint_id)
            .hset(
                checkpoint_proofs_key,
                id.to_fixed_bytes().to_vec(),
                proof_bytes,
            )
            .hset(
                public_inputs_key,
                id.to_fixed_bytes().to_vec(),
                public_inputs_data,
            )
            .execute_atomic(&self.redis).await?;

        Ok(())
    }
    async fn set_bytes_by_id_batch(
        &self,
        kv_pairs: &[KVQPair<QProvingJobDataID, Vec<u8>>],
    ) -> anyhow::Result<()> {
        if kv_pairs.is_empty() {
            return Ok(());
        }

        let mut builder = self.redis.cmd_builder();

        for kv in kv_pairs.iter() {
            let checkpoint_id = kv.key.goal_id;
            let checkpoint_list_key = self.checkpoint_list_key();
            let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);

            builder = builder
                .sadd(checkpoint_list_key, checkpoint_id)
                .hset(
                    checkpoint_proofs_key,
                    kv.key.to_fixed_bytes().to_vec(),
                    &kv.value
                );
        }

        builder.execute_atomic(&self.redis).await?;
        Ok(())
    }
    async fn set_bytes_by_id(&self, id: QProvingJobDataID, data: &[u8]) -> anyhow::Result<()> {
        let checkpoint_id = id.goal_id;

        self.add_checkpoint(checkpoint_id).await?;

        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);
        self.redis.hset(
            checkpoint_proofs_key,
            id.to_fixed_bytes().to_vec(),
            data.to_vec(),
        ).await?;
        Ok(())
    }

    async fn inc_counter_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<u32> {
        let counter_value = self.redis.hincr(
            self.proof_store_counters_key(),
            id.to_fixed_bytes().to_vec(),
            1,
        ).await?;
        Ok(counter_value as u32)
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
        let checkpoint_list_key = self.checkpoint_list_key();
        let threshold = current_height.saturating_sub(keep_blocks);

        let checkpoints_to_remove: Vec<u64> = self.redis.smembers(checkpoint_list_key.clone()).await?
            .into_iter()
            .filter(|&checkpoint_id| checkpoint_id < threshold)
            .collect();

        if !checkpoints_to_remove.is_empty() {
            let mut builder = self.redis.cmd_builder();

            for checkpoint_id in &checkpoints_to_remove {
                let checkpoint_proofs_key = self.checkpoint_proofs_key(*checkpoint_id);
                builder = builder
                    .del(checkpoint_proofs_key)
                    .srem(&checkpoint_list_key, &[*checkpoint_id]);
            }

            builder.execute_atomic(&self.redis).await?;
        }

        Ok(())
    }

    async fn clear(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        let mut builder = self.redis.cmd_builder();
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);
        let checkpoint_list_key = self.checkpoint_list_key();
        builder = builder
            .del(checkpoint_proofs_key)
            .srem(&checkpoint_list_key, &[checkpoint_id]);
        builder.execute_atomic(&self.redis).await?;
        Ok(())
    }
}

#[async_trait]
impl CheckpointDrainQueueEmitterAsyncImm for ProofStoreRedisAsync {
    async fn cdq_push_imm<T: DQSerializable>(&self, item: T) -> anyhow::Result<()> {
        let channel_id = item.get_dq_metadata().channel_id;
        let item_id = item.get_dq_metadata().item_id;//user id or realm id
        let key = self.drain_queue_key(channel_id);
        let id_key = self.id_key(channel_id);
        let now = self.redis.server_time().await?;
        let mut builder = self.redis.cmd_builder();
        let builder = builder.hset(
            key,
            item_id,
            item.to_bytes()?,
        ).zadd(id_key, item_id, now[0]);
        builder.execute_atomic(&self.redis).await?;
        Ok(())
    }
}

#[async_trait]
impl CheckpointDrainQueueConsumerAsyncImm for ProofStoreRedisAsync {
    async fn cdq_drain_imm<T: DQSerializable>(
        &self,
        channel_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let key = self.drain_queue_key(channel_id);
        let id_key = self.id_key(channel_id);
        let ids: Vec<u64> = self.redis.zrange(id_key.clone(), 0, -1).await?;
        self.redis.del(id_key).await?;
        // Handle empty ids case
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let members: Vec<Vec<u8>> = self.redis.hget(key.clone(), ids).await?;
        let items: Vec<T> = members
            .into_iter()
            .filter_map(|data| T::from_bytes(&data).ok())
            .collect();
        self.redis.del(key).await?;
        Ok(items)
    }

    async fn cdq_peek_imm<T: DQSerializable>(
        &self,
        channel_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let id_key: String = self.id_key(channel_id);
        let ids: Vec<u64> = self.redis.zrange(id_key.clone(), 0, -1).await?;
        // Handle empty ids case
        if ids.is_empty() {
            self.redis.del(id_key).await?;
            return Ok(vec![]);
        }
        let members: Vec<Vec<u8>> = self.redis.hget(self.drain_queue_key(channel_id), ids).await?;
        let items: Vec<T> = members
            .into_iter()
            .filter_map(|data| T::from_bytes(&data).ok())
            .collect();

        Ok(items)
    }

    async fn cdq_len_imm(
        &self,
        channel_id: u64,
    ) -> anyhow::Result<usize> {
        let id_key = self.id_key(channel_id);
        let count: usize = self.redis.zcard(id_key).await?;
        Ok(count)
    }
}

#[async_trait]
pub trait CheckpointDrainQueueConsumerAsyncImmWithPosition: CheckpointDrainQueueConsumerAsyncImm {
    async fn peek_with_position<T: KVQSerializable + Send + Sync>(
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
        match self.redis_blocking.blpop(self.worker_queue_key(), 0).await? {
            Some((_, data)) => {
                Ok(QProvingJobDataID::try_from_byte_vec(&data)?)
            }
            None => {
                Err(anyhow::anyhow!("BLPOP returned None with infinite timeout"))
            }
        }
    }
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        if jobs.is_empty() {
            return Ok(());
        }

        let job_bytes: Vec<u8> = jobs.iter()
            .flat_map(|job| job.to_fixed_bytes().to_vec())
            .collect();

        self.redis.rpush(self.worker_queue_key(), job_bytes).await?;
        Ok(())
    }
    async fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()> {
        self.redis.rpush(self.notifications_queue_key(), job.to_fixed_bytes().to_vec()).await?;
        Ok(())
    }
}

#[async_trait]
impl WorkerEventTransmitterAsyncImm for ProofStoreRedisAsync {
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()> {
        if jobs.is_empty() {
            return Ok(());
        }

        let job_bytes: Vec<u8> = jobs.iter()
            .flat_map(|job| job.to_fixed_bytes().to_vec())
            .collect();

        self.redis.rpush(self.worker_queue_key(), job_bytes).await?;
        Ok(())
    }
    async fn wait_for_block_proving_jobs_imm(
        &self,
        _checkpoint_id: u64,
        timeout: Option<Duration>,
    ) -> anyhow::Result<QProvingJobDataID> {
        let timeout_secs = timeout.map(|d| d.as_secs() as usize).unwrap_or(0);

        match self.redis_blocking.blpop(self.notifications_queue_key(), timeout_secs).await? {
            Some((_, data)) => {
                Ok(QProvingJobDataID::try_from_byte_vec(&data)?)
            }
            None => {
                Err(anyhow::anyhow!("Timeout waiting for block proving job"))
            }
        }
    }

    async fn wait_for_job_proof<C: GenericConfig<D> + 'static, const D: usize>(
        &self,
        job_id: QProvingJobDataID,
        timeout: Option<Duration>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>
    where
        C::Hasher: plonky2::plonk::config::AlgebraicHasher<C::F>,
    {
        let start_time = std::time::Instant::now();
        let timeout_duration = timeout.unwrap_or(Duration::from_secs(300));

        loop {
            if start_time.elapsed() > timeout_duration {
                return Err(anyhow::anyhow!("Timeout waiting for job proof"));
            }

            if let Ok(proof) = self.get_proof_by_id(job_id).await {
                return Ok(proof);
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[async_trait]
impl CheckpointHistoryQueueEmitterAsyncImm for ProofStoreRedisAsync {
    async fn chq_push_imm<T: HQSerializable>(&self, item: T) -> anyhow::Result<()> {
        let metadata = item.get_hq_metadata();
        let bytes = item.to_bytes()?;

        let key = format!(
            "{}-{}_{}",
            self.checkpoint_history_queue_prefix_key(),
            metadata.channel_id,
            metadata.checkpoint_id
        );

        let current_checkpoint_key = format!(
            "{}-{}",
            self.checkpoint_history_queue_prefix_key(),
            metadata.channel_id
        );

        self.redis.cmd_builder()
            .set(key, bytes)
            .set(current_checkpoint_key, metadata.checkpoint_id)
            .execute_atomic(&self.redis).await?;

        Ok(())
    }
}

#[async_trait]
impl CheckpointHistoryQueueConsumerAsyncImm for ProofStoreRedisAsync {
    async fn chq_items_gte<T: HQSerializable>(
        &self,
        channel_id: u64,
        start_checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        let current_checkpoint_key = format!(
            "{}-{}",
            self.checkpoint_history_queue_prefix_key(),
            channel_id
        );

        let cur_checkpoint_id_opt: Option<u64> = self.redis.get::<_, Option<u64>>(current_checkpoint_key).await.ok().flatten();

        if let Some(current_id) = cur_checkpoint_id_opt {
            if current_id >= start_checkpoint_id {
                let mut results = Vec::with_capacity((current_id - start_checkpoint_id + 1) as usize);

                for i in start_checkpoint_id..=current_id {
                    let item_key = format!(
                        "{}-{}_{}",
                        self.checkpoint_history_queue_prefix_key(),
                        channel_id,
                        i
                    );

                    if let Ok(result_bytes) = self.redis.get::<_, Vec<u8>>(item_key).await {
                        if let Ok(item) = T::from_bytes(&result_bytes) {
                            results.push(item);
                        }
                    }
                }

                return Ok(results);
            }
        }

        Ok(vec![])
    }
    async fn wait_for_next_item_imm<T: HQSerializable>(
        &self,
        channel_id: u64,
        start_checkpoint_id: u64,
    ) -> anyhow::Result<T> {
        loop {
            let current_checkpoint_key = format!(
                "{}-{}",
                self.checkpoint_history_queue_prefix_key(),
                channel_id
            );

            let cur_checkpoint_id_opt: Option<u64> = self.redis.get::<_, Option<u64>>(current_checkpoint_key).await.ok().flatten();

            if let Some(current_id) = cur_checkpoint_id_opt {
                if current_id >= start_checkpoint_id {
                    let item_key = format!(
                        "{}-{}_{}",
                        self.checkpoint_history_queue_prefix_key(),
                        channel_id,
                        current_id
                    );

                    if let Ok(result_bytes) = self.redis.get::<_, Vec<u8>>(item_key).await {
                        if let Ok(item) = T::from_bytes(&result_bytes) {
                            return Ok(item);
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
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
        let key = format!("{}-{}", self.notifications_queue_key(), item.get_hq_metadata().channel_id);
        tracing::info!("🔄 Producing item to key: {}", key);
        self.redis.rpush(key, item.to_bytes()?).await?;
        Ok(())
    }

    async fn consume_item(&self, channel_id: u64) -> anyhow::Result<T> {
        let key = format!("{}-{}", self.notifications_queue_key(), channel_id);
        tracing::info!("🔍 Consuming from key: {}", key);

        loop {
            match self.redis.lpop::<_, Vec<u8>>(key.clone(), None).await {
                Ok(Some(data)) => {
                    match T::from_bytes(&data) {
                        Ok(item) => return Ok(item),
                        Err(err) => return Err(anyhow::anyhow!("Failed to parse item: {:?}", err)),
                    }
                }
                Ok(None) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Err(err) => {
                    tracing::warn!("Redis lpop error, retrying: {:?}", err);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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

        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");
        let mut builder = self.redis.cmd_builder();

        for user in pending_users.iter() {
            let user_bytes = bincode::serialize(user).map_err(|e| anyhow::anyhow!("Failed to serialize user: {}", e))?;
            builder = builder.rpush(key.clone(), user_bytes);
        }

        builder.execute_atomic(&self.redis).await?;
        Ok(())
    }

    async fn pop_pending_users<F: RichField>(
        &self,
        count: usize,
    ) -> anyhow::Result<Vec<MerkleProofCore<QHashOut<F>>>> {
        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");

        let bytes_vec: Option<Vec<Vec<u8>>> = self.redis.lpop(key, Some(count)).await?;

        let mut users = Vec::new();
        if let Some(bytes_list) = bytes_vec {
            for bytes in bytes_list {
                let user = bincode::deserialize::<MerkleProofCore<QHashOut<F>>>(&bytes)
                    .map_err(|e| anyhow::anyhow!("Deserialization failed for pending user: {}", e))?;
                users.push(user);
            }
        }

        Ok(users)
    }

    async fn get_pending_users_count(&self) -> anyhow::Result<usize> {
        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");
        let length: usize = self.redis.llen(key).await?;
        Ok(length)
    }

    async fn peek_with_position<F: RichField>(
        &self,
        count: usize,
        checkpoint_id: u64,
    ) -> anyhow::Result<(Vec<MerkleProofCore<QHashOut<F>>>, QueueOffsetState)> {
        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");

        let start_position = 0i64;
        let items: Vec<Vec<u8>> = self.redis.lrange(key, start_position as isize, (count - 1) as isize).await?;

        let mut users = Vec::with_capacity(items.len());
        for item_bytes in items {
            let user = bincode::deserialize::<MerkleProofCore<QHashOut<F>>>(&item_bytes)
                .map_err(|e| anyhow::anyhow!("Deserialization failed for pending user: {}", e))?;
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

        let state_key = format!("{}-{}", self.realm_pending_user_key(), "CONSUMPTION_STATE");
        let state_data = bincode::serialize(&state)
            .map_err(|e| anyhow::anyhow!("Failed to serialize state: {}", e))?;
        self.redis.set(state_key, state_data).await?;

        tracing::debug!("Consumed {} users from positions {}-{} for checkpoint {}",
              users.len(), start_position, end_position, checkpoint_id);

        Ok((users, state))
    }

    async fn commit_offset(&self, state: &QueueOffsetState) -> anyhow::Result<()> {
        if state.consumed_count == 0 {
            return Ok(());
        }

        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");
        self.redis.ltrim(key, (state.end_position + 1) as isize, -1).await?;

        let state_key = format!("{}-{}", self.realm_pending_user_key(), "CONSUMPTION_STATE");
        self.redis.del(state_key).await?;

        tracing::debug!("Committed consumption of {} users for checkpoint {}",
              state.consumed_count, state.checkpoint_id);

        Ok(())
    }

    async fn get_last_peek_offset(&self) -> anyhow::Result<Option<QueueOffsetState>> {
        let state_key = format!("{}-{}", self.realm_pending_user_key(), "CONSUMPTION_STATE");

        let state_data: Option<Vec<u8>> = self.redis.get(state_key).await.ok();
        if let Some(data) = state_data {
            if !data.is_empty() {
                let state: QueueOffsetState = bincode::deserialize(&data).map_err(|e| anyhow::anyhow!("Failed to deserialize state: {}", e))?;
                return Ok(Some(state))
            }
        }
        Ok(None)
    }
}


#[async_trait]
impl CheckpointDrainQueueConsumerAsyncImmWithPosition for ProofStoreRedisAsync {
    async fn peek_with_position<T: KVQSerializable + Send + Sync>(
        &self,
        count: Option<isize>,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<(Vec<T>, QueueOffsetState)> {
        let key = self.drain_queue_key(channel_id);

        let end_index = match count {
            Some(c) if c > 0 => c - 1,
            Some(_) => -1,
            None => -1,
        };
        let ids: Vec<u64> = self.redis.zrange(self.id_key(channel_id), 0, end_index).await?;
        // Handle empty ids case
        if ids.is_empty() {
            return Ok((vec![], QueueOffsetState {
                start_position: 0,
                end_position: -1,
                checkpoint_id,
                channel_id,
                consumed_count: 0,
            }));
        }
        let members: Vec<Vec<u8>> = self.redis.hget(key, ids).await?;

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

        let state_key = format!("{}-{}-{}", self.worker_queue_key(), "DRAIN_CONSUMPTION_STATE", channel_id);
        let state_data = bincode::serialize(&state).map_err(|e| anyhow::anyhow!("Failed to serialize state: {}", e))?;
        self.redis.set(state_key.clone(), state_data).await?;

        tracing::debug!("Consumed redis {} items from drain queue {} for checkpoint {}, state_key {}",
              items.len(), channel_id, checkpoint_id, state_key);

        Ok((items, state))
    }

    async fn get_last_peek_offset(
        &self,
        channel_id: u64,
    ) -> anyhow::Result<Option<QueueOffsetState>> {
        let state_key = format!("{}-{}-{}", self.worker_queue_key(), "DRAIN_CONSUMPTION_STATE", channel_id);
        let state_data: Option<Vec<u8>> = self.redis.get(state_key).await.ok();
        if let Some(data) = state_data {
            if !data.is_empty() {
                let state: QueueOffsetState = bincode::deserialize(&data).map_err(|e| anyhow::anyhow!("Failed to deserialize state: {}", e))?;
                return Ok(Some(state))
            }
        }
        Ok(None)
    }

    async fn commit_offset(&self, state: &QueueOffsetState) -> anyhow::Result<()> {
        if state.consumed_count == 0 {
            return Ok(());
        }
        let state_key = format!("{}-{}-{}", self.worker_queue_key(), "DRAIN_CONSUMPTION_STATE", state.channel_id);
        let mut builder = self.redis.cmd_builder();
        let key = self.drain_queue_key(state.channel_id);
        let id_key = self.id_key(state.channel_id);
        let ids: Vec<u64> = self.redis.zrange(id_key.clone(), state.start_position as isize, state.end_position as isize).await?;
        builder = builder.zremrangebyrank(id_key.clone(), state.start_position as isize, state.end_position as isize);
        builder = builder.hdel(key, &ids);// remove tx
        builder = builder.del(state_key.clone());
        tracing::debug!("Consumed redis {} items for checkpoint {}, channel_id {}, state_key {}",
                         state.consumed_count, state.checkpoint_id, state.channel_id, state_key);
        let now = self.redis.server_time().await?;
        builder = builder.zremrangebyscore(id_key, 0, now[0] - 1800);
        builder.execute_atomic(&self.redis).await?;
        Ok(())
    }
}
