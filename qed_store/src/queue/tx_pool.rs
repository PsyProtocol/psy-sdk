use async_trait::async_trait;
use chrono::Utc;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use kvq::traits::KVQSerializable;
use qed_core::job::id::QProvingJobDataID;
use qed_data::guta::api::UserEndCapNonProofCoreInputQueueItem;
use crate::queue::{ProofStoreRedisAsync, QueuePrefixKey, PS_DRAIN_QUEUE_KEY_PREFIX};
use crate::queue::redis_queue::{CheckpointDrainQueueConsumerAsyncImmWithPosition, QueueOffsetState};


#[async_trait]
pub trait TxPoolAsyncImm: CheckpointDrainQueueConsumerAsyncImmWithPosition + Send + Sync {
    async fn contains_tx_id(&self, id: QProvingJobDataID) -> anyhow::Result<bool>;
    async fn add_user_tx<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
        user_end_cap: UserEndCapNonProofCoreInputQueueItem<C::F>,
    ) -> anyhow::Result<()>;

    async fn get_user_txs<C: GenericConfig<D>, const D: usize>(
        &self,
        count: Option<isize>,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<UserEndCapNonProofCoreInputQueueItem<C::F>>>;

    async fn remove_user_txs(&self, channel_id: u64) -> anyhow::Result<()>;
    async fn pool_len(&self, channel_id: u64) -> anyhow::Result<usize> {
        self.cdq_len_imm(channel_id).await
    }
}


#[async_trait]
impl TxPoolAsyncImm for ProofStoreRedisAsync {
    async fn contains_tx_id(&self, id: QProvingJobDataID) -> anyhow::Result<bool> {
        self.redis.exists(id.to_fixed_bytes().to_vec()).await
    }
    async fn add_user_tx<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
        user_end_cap: UserEndCapNonProofCoreInputQueueItem<C::F>,
    ) -> anyhow::Result<()> {
        let mut builder = self.redis.cmd_builder();
        let checkpoint_id = id.goal_id;
        let checkpoint_list_key = self.checkpoint_list_key();
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);
        let public_inputs_key = self.public_inputs_key();

        let proof_bytes = bincode::serialize(proof)?;
        let public_inputs_data = bincode::serialize(&proof.public_inputs)?;
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
            ).set_ex(
                id.to_fixed_bytes().to_vec(),   //QProvingJobDataID
                2 * 60,                //2 minutes
                user_end_cap.to_bytes()?,//UserEndCapNonProofCoreInputQueueItem
            );
        // QProvingJobDataID
        let (key, bytes) = self.cdq_kv_pair(user_end_cap.channel_id, id)?;
        builder = builder.rpush(key, bytes);
        builder.execute_atomic(&self.redis).await?;
        Ok(())
    }

    async fn get_user_txs<C: GenericConfig<D>, const D: usize>(
        &self,
        count: Option<isize>,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<UserEndCapNonProofCoreInputQueueItem<C::F>>> {
        let (job_ids, state) = CheckpointDrainQueueConsumerAsyncImmWithPosition::peek_with_position::<QProvingJobDataID>(self, count, channel_id, checkpoint_id).await?;
        if job_ids.is_empty() {
            return Ok(vec![]);
        }
        let mut ids: Vec<Vec<u8>> = job_ids.iter().map(|id| id.to_fixed_bytes().to_vec()).collect();
        ids.dedup(); //remove duplicates
        //TODO remove old user tx
        let rets: Option<Vec<Option<Vec<u8>>>> = self.redis.mget(ids).await.ok();
        let mut txs = vec![];
        if let Some(rets) = rets {
            for item in &rets {
                if let Some(ret) = item {
                    let txc = UserEndCapNonProofCoreInputQueueItem::from_bytes(ret)?;
                    txs.push(txc);
                }
            }
        }
        Ok(txs)
    }

    async fn remove_user_txs(&self, channel_id: u64) -> anyhow::Result<()> {
        let state = CheckpointDrainQueueConsumerAsyncImmWithPosition::get_last_peek_offset(self, channel_id).await?;
        if let Some(state) = state {
            // CheckpointDrainQueueConsumerAsyncImmWithPosition::commit_offset(self, &state).await?;
            // remove txs
            // remove user end cap
            if state.consumed_count == 0 {
                return Ok(());
            }

            let checkpoint_queue_prefix =
                format!("{}-{}", self.worker_queue_key(), PS_DRAIN_QUEUE_KEY_PREFIX);
            let key = format!("{}-{}", checkpoint_queue_prefix, state.channel_id);
            let state_key = format!("{}-{}-{}", self.worker_queue_key(), "DRAIN_CONSUMPTION_STATE", state.channel_id);
            let job_ids: Vec<Vec<u8>> = self.redis.lrange(key.clone(), state.start_position as isize, state.end_position as isize).await?;
            let mut builder = self.redis.cmd_builder();
            if job_ids.len() <= 1000 {
                for job_id in &job_ids {
                    builder = builder.del(job_id);
                }
                let checkpoint_proofs_key = self.checkpoint_proofs_key(state.checkpoint_id);
                builder = builder.hdel(checkpoint_proofs_key, &job_ids);
                // let public_inputs_key = self.public_inputs_key();
                // builder = builder.hdel(public_inputs_key, &job_ids);
            }
            builder.ltrim(key, (state.end_position + 1) as isize, -1)
                .del(state_key.clone())
                .execute_atomic(&self.redis).await?;
            tracing::debug!("Remove user tx {} items for checkpoint {}, channel_id {}, state_key {}",
              state.consumed_count, state.checkpoint_id, state.channel_id, state_key);
        }
        Ok(())
    }
}

#[async_trait]
pub trait TxPoolAsyncImmV2: Send + Sync {
    async fn contains_tx(&self, channel_id: u64, id: u64) -> anyhow::Result<bool>;
    async fn add_tx<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
        user_end_cap: UserEndCapNonProofCoreInputQueueItem<C::F>,
    ) -> anyhow::Result<()>;

    async fn get_txs<C: GenericConfig<D>, const D: usize>(
        &self,
        count: Option<isize>,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<UserEndCapNonProofCoreInputQueueItem<C::F>>>;

    async fn remove_txs(&self, channel_id: u64) -> anyhow::Result<()>;

    async fn remove_expired_txs(&self, channel_id: u64) -> anyhow::Result<()>;
    async fn len(&self, channel_id: u64) -> anyhow::Result<usize>;
}

#[async_trait]
impl TxPoolAsyncImmV2 for ProofStoreRedisAsync {
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

    async fn add_tx<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
        tx: UserEndCapNonProofCoreInputQueueItem<C::F>,
    ) -> anyhow::Result<()> {
        let checkpoint_id = id.goal_id;
        let checkpoint_list_key = self.checkpoint_list_key();
        let checkpoint_proofs_key = self.checkpoint_proofs_key(checkpoint_id);
        let tx_key = self.tx_key(tx.channel_id);
        let id_key = self.id_key(tx.channel_id);
        let public_inputs_key = self.public_inputs_key();

        let proof_bytes = bincode::serialize(proof)?;
        let public_inputs_data = bincode::serialize(&proof.public_inputs)?;
        let now = self.redis.server_time().await?;
        let mut builder = self.redis.cmd_builder();
        let user_id = tx.cst_user_update.user_id;
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
                user_id,
                tx.to_bytes()?,
            ).zadd(
                id_key.clone(),
                user_id,
                now[0],
            );
        builder.execute_atomic(&self.redis).await?;
        Ok(())
    }

    async fn get_txs<C: GenericConfig<D>, const D: usize>(
        &self,
        count: Option<isize>,
        channel_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<Vec<UserEndCapNonProofCoreInputQueueItem<C::F>>> {
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
        let mut txs:Vec<UserEndCapNonProofCoreInputQueueItem<C::F>> = vec![];
        for (i, user_end_cap_data) in txs_data.iter().enumerate() {
            if let Some(user_end_cap_data) = user_end_cap_data {
                let user_end_cap = UserEndCapNonProofCoreInputQueueItem::<C::F>::from_bytes(user_end_cap_data)?;
                txs.push(user_end_cap);
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

    async fn len(&self, channel_id: u64) -> anyhow::Result<usize> {
        self.redis.zcard(self.id_key(channel_id)).await
    }

    async fn remove_expired_txs(&self, channel_id: u64) -> anyhow::Result<()> {
        let id_key = self.id_key(channel_id);
        let now = self.redis.server_time().await?;
        self.redis.zremrangebyscore(id_key, 0, now[0] - 1800).await
    }
}