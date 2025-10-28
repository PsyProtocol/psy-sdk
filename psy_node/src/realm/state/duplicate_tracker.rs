use anyhow::Result;
use async_trait::async_trait;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;

use crate::{common_v2::traits::realm::UniqueQueueId, realm::state::queue_traits::DuplicateTracker};

const USER_SUBMISSION_TTL_SECONDS: u64 = 3600; // 1 hours

pub struct RedisDuplicateTracker {
    redis_pool: Pool<RedisConnectionManager>,
    realm_id: u32,
    ttl_seconds: u64,
}

impl RedisDuplicateTracker {
    pub fn new(redis_pool: Pool<RedisConnectionManager>, realm_id: u32) -> Self {
        Self {
            redis_pool,
            realm_id,
            ttl_seconds: USER_SUBMISSION_TTL_SECONDS,
        }
    }

    fn get_key(&self, checkpoint: UniqueQueueId, user_id: u64) -> String {
        format!("submitted:{}:{}:{}:{}", self.realm_id, checkpoint.id, checkpoint.uuid, user_id)
    }
}

#[async_trait]
impl DuplicateTracker for RedisDuplicateTracker {
    async fn has_submitted(&self, checkpoint: UniqueQueueId, user_id: u64) -> Result<bool> {
        let key = self.get_key(checkpoint, user_id);
        let mut conn = self.redis_pool.get().await?;
        Ok(conn.exists(&key).await?)
    }

    async fn mark_submitted(&self, checkpoint: UniqueQueueId, user_id: u64) -> Result<()> {
        let key = self.get_key(checkpoint, user_id);
        let mut conn = self.redis_pool.get().await?;
        conn.set_ex(&key, "1", self.ttl_seconds).await?;
        Ok(())
    }

    async fn clear_checkpoint(&self, checkpoint: UniqueQueueId) -> Result<()> {
        let pattern = format!("submitted:{}:{}:{}:*", self.realm_id, checkpoint.id, checkpoint.uuid);
        let mut conn = self.redis_pool.get().await?;
        let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query_async(&mut *conn).await?;

        if !keys.is_empty() {
            redis::cmd("DEL").arg(&keys).query_async(&mut *conn).await?;
        }
        Ok(())
    }
}
