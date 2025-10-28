use std::{marker::PhantomData, sync::Arc};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use plonky2::hash::hash_types::RichField;
use psy_data::config::store_config::StagingCheckpointInfoStore;
use psy_store::{
    queue::{new_redis_async_pool, QueueId, RsmqQueue},
    store::QEDStore,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
    common_v2::traits::realm::{RealmEdgeUserUpdateSubmission, RealmProcessorEdgeQueueHelper, UniqueQueueId},
    realm::state::queue_traits::{DuplicateTracker, EdgeSubmissionQueue},
};

const USER_SUBMISSION_TTL_SECONDS: u64 = 3600;
const USER_EDGE_STATE_TTL_SECONDS: u64 = 3600;

pub struct RealmEdgeQueueHelper<F: RichField> {
    queue: Arc<dyn EdgeSubmissionQueue<F>>,
    tracker: Arc<dyn DuplicateTracker>,
    queue_uuid: Arc<RwLock<u128>>,
    redis_pool: Pool<RedisConnectionManager>,
    store: Arc<QEDStore>,
    _phantom: PhantomData<F>,
}

impl<F: RichField> RealmEdgeQueueHelper<F> {
    pub async fn new(
        queue: Arc<dyn EdgeSubmissionQueue<F>>,
        tracker: Arc<dyn DuplicateTracker>,
        redis_url: &str,
        pool_size: usize,
        store: Arc<QEDStore>,
    ) -> Self {
        let redis_pool = match new_redis_async_pool(redis_url, pool_size).await {
            Ok(pool) => pool,
            Err(e) => panic!("Failed to create Redis pool: {}", e),
        };

        Self {
            queue,
            tracker,
            queue_uuid: Arc::new(RwLock::new(0)),
            redis_pool,
            _phantom: PhantomData,
            store,
        }
    }
    pub async fn get_shared_checkpoint_id(&self) -> Result<UniqueQueueId> {
        if let Some((uuid, checkpoint_id, _info)) = StagingCheckpointInfoStore::<QEDStore>::get_latest_checkpoint_info_with_uuid(&self.store)? {
            Ok(UniqueQueueId { id: checkpoint_id, uuid })
        } else {
            anyhow::bail!("No staging checkpoint info found")
        }
    }

    pub async fn has_user_submitted(&self, checkpoint: UniqueQueueId, user_id: u64) -> Result<bool> {
        self.tracker.has_submitted(checkpoint, user_id).await
    }

    pub async fn mark_user_submitted(&self, checkpoint: UniqueQueueId, user_id: u64) -> Result<()> {
        self.tracker.mark_submitted(checkpoint, user_id).await
    }

    pub async fn enqueue_submission(&self, checkpoint: UniqueQueueId, submission: RealmEdgeUserUpdateSubmission<F>) -> Result<()> {
        self.queue.enqueue(checkpoint, submission).await
    }

    pub async fn enqueue_batch(&self, checkpoint: UniqueQueueId, submissions: Vec<RealmEdgeUserUpdateSubmission<F>>) -> Result<()> {
        self.queue.enqueue_batch(checkpoint, submissions).await
    }
    pub async fn set_key(&self, key: &str, value: u128) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;

        // Store as string for u128
        conn.set_ex(key, value.to_string(), USER_EDGE_STATE_TTL_SECONDS)
            .await
            .map_err(|e| anyhow!("Failed to set key {}: {}", key, e))?;

        Ok(())
    }
    pub async fn get_key(&self, key: &str) -> Result<u128> {
        let mut conn = self.redis_pool.get().await?;

        let value: Option<String> = conn.get(key).await.map_err(|e| anyhow!("Failed to get key {}: {}", key, e))?;

        match value {
            Some(s) => s.parse::<u128>().map_err(|e| anyhow!("Failed to parse value for key {}: {}", key, e)),
            None => Err(anyhow!("Key {} not found", key)),
        }
    }
}

impl<F> RealmProcessorEdgeQueueHelper<F> for RealmEdgeQueueHelper<F>
where
    F: RichField,
{
    async fn dump_user_updates(&self, queue_id: UniqueQueueId) -> Result<Vec<RealmEdgeUserUpdateSubmission<F>>> {
        let submissions = self.queue.drain_all(queue_id).await?;
        self.tracker.clear_checkpoint(queue_id).await?;
        tracing::info!("Dumped {} submissions for checkpoint {:?}", submissions.len(), queue_id);

        Ok(submissions)
    }

    async fn has_user_updates(&self, queue_id: UniqueQueueId) -> Result<bool> {
        self.queue.has_messages(&queue_id).await
    }

    async fn get_user_updates(&self, queue_id: UniqueQueueId) -> Result<Vec<RealmEdgeUserUpdateSubmission<F>>> {
        self.queue.peek_all(queue_id).await
    }
}
