use async_trait::async_trait;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_core::job::id::QProvingJobDataID;
use crate::queue::{ProofStoreRedisAsync, QueuePrefixKey};
use crate::queue::redis_queue::{QueueOffsetState};
use qed_core::job::drain_queue::DQSerializable;

#[async_trait]
pub trait TxPoolAsyncImm: Send + Sync {
    async fn contains_tx(&self, channel_id: u64, id: u64) -> anyhow::Result<bool>;
    async fn add_tx<C: GenericConfig<D>, const D: usize, T: DQSerializable>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
        tx: T,
    ) -> anyhow::Result<()>;

    async fn get_txs<C: GenericConfig<D>, const D: usize, T: DQSerializable>(
        &self,
        count: Option<isize>,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>>;

    async fn remove_txs(&self, channel_id: u64) -> anyhow::Result<()>;

    async fn remove_expired_txs(&self, channel_id: u64) -> anyhow::Result<()>;
    async fn len(&self, channel_id: u64) -> anyhow::Result<usize>;
}

#[async_trait]
impl TxPoolAsyncImm for ProofStoreRedisAsync {
    async fn contains_tx(&self, channel_id: u64, id: u64) -> anyhow::Result<bool> {
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

    async fn add_tx<C: GenericConfig<D>, const D: usize, T: DQSerializable>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
        tx: T,
    ) -> anyhow::Result<()> {
        let checkpoint_id = id.goal_id;
        let checkpoint_list_key = self.checkpoint_list_key();
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);
        let tx_key = self.tx_key(tx.get_dq_metadata().channel_id);
        let id_key = self.id_key(tx.get_dq_metadata().channel_id);
        let public_inputs_key = self.public_inputs_key();

        let proof_bytes = bincode::serialize(proof)?;
        let public_inputs_data = bincode::serialize(&proof.public_inputs)?;
        let now = self.redis.server_time().await?;
        let mut builder = self.redis.cmd_builder();
        let item_id = tx.get_dq_metadata().item_id;//user id or realm id
        builder = builder
            .sadd(checkpoint_list_key, checkpoint_id)
            .hset(
                checkpoint_proofs_key,
                id.to_fixed_bytes().to_vec(),
                proof_bytes,
            ).hset(
                public_inputs_key,
                id.to_fixed_bytes().to_vec(),
                public_inputs_data,
            ).hset(
                tx_key,
                item_id,
                tx.to_bytes()?,
            ).zadd(
                id_key.clone(),// zset name
                item_id,
                now[0], // score
            );
        builder.execute_atomic(&self.redis).await?;
        Ok(())
    }

    async fn get_txs<C: GenericConfig<D>, const D: usize, T: DQSerializable>(
        &self,
        count: Option<isize>,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<T>> {
        self.remove_expired_txs(channel_id).await?;
        let len= self.len(channel_id).await?;
        if len == 0 {
            return Ok(vec![]);
        }
        let start_position: i64 = 0;
        let mut stop = -1;
        if let Some(count) = count {
            if count < len as isize {
                stop = count - 1;
            }
        }

        let ids: Vec<u64> = self.redis.zrange(self.id_key(channel_id), start_position as isize, stop).await?;
        let txs_data: Vec<Option<Vec<u8>>> = self.redis.hget(self.tx_key(channel_id), ids).await?;
        let mut txs:Vec<T> = vec![];
        for (i, tx_data) in txs_data.iter().enumerate() {
            if let Some(tx_data) = tx_data {
                let tx = T::from_bytes(tx_data)?;
                txs.push(tx);
            }
        }

        let state = QueueOffsetState {
            start_position,
            end_position: stop as i64,
            checkpoint_id,
            channel_id,
            consumed_count: txs.len(),
        };

        let state_key = format!("{}-{}-{}", self.worker_queue_key(), "DRAIN_CONSUMPTION_STATE", channel_id);
        let state_data = bincode::serialize(&state).map_err(|e| anyhow::anyhow!("Failed to serialize state: {}", e))?;
        self.redis.set(state_key.clone(), state_data).await?;

        tracing::debug!("Consumed redis {} items from drain queue {} for checkpoint {}, state_key {}",
              txs.len(), channel_id, checkpoint_id, state_key);

        Ok(txs)
    }

    async fn remove_txs(&self, channel_id: u64) -> anyhow::Result<()> {
        let state_key = format!("{}-{}-{}", self.worker_queue_key(), "DRAIN_CONSUMPTION_STATE", channel_id);
        let state_data: Option<Vec<u8>> = self.redis.get(state_key.clone()).await.ok();
        let mut builder = self.redis.cmd_builder();
        let id_key = self.id_key(channel_id);
        if let Some(data) = state_data {
            if !data.is_empty() {
                let state: QueueOffsetState = bincode::deserialize(&data).map_err(|e| anyhow::anyhow!("Failed to deserialize state: {}", e))?;
                let tx_key = self.tx_key(channel_id);
                let id_key = self.id_key(channel_id);
                let ids: Vec<u64> = self.redis.zrange(id_key.clone(), state.start_position as isize, state.end_position as isize).await?;
                builder = builder.zremrangebyrank(id_key, state.start_position as isize, state.end_position as isize);
                builder = builder.hdel(tx_key, &ids);// remove tx
                builder = builder.del(state_key.clone());
            }
        }
        let now = self.redis.server_time().await?;
        builder = builder.zremrangebyscore(id_key, 0, now[0] - 1800);
        builder.execute_atomic(&self.redis).await?;
        Ok(())
    }

    async fn remove_expired_txs(&self, channel_id: u64) -> anyhow::Result<()> {
        let id_key = self.id_key(channel_id);
        let now = self.redis.server_time().await?;
        self.redis.zremrangebyscore(id_key, 0, now[0] - 1800).await
    }

    async fn len(&self, channel_id: u64) -> anyhow::Result<usize> {
        self.redis.zcard(self.id_key(channel_id)).await
    }
}