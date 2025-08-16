use async_trait::async_trait;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use redis::AsyncCommands;
use super::redis_queue::BizKey;

pub const REALM_PENDING_USER_QUEUE_KEY_PREFIX: &'static str = "RMPUQ";

#[async_trait]
pub trait QPendingUserStoreAsyncImm: Send + Sync {
    async fn push_pending_users<F: RichField>(
        &self,
        pending_users: &[MerkleProofCore<QHashOut<F>>],
    ) -> anyhow::Result<()>;
    async fn get_current_pending_user_index(&self) -> anyhow::Result<usize>;
    async fn set_current_pending_user_index(&self, index: usize) -> anyhow::Result<()>;
    async fn get_total_pending_users(&self) -> anyhow::Result<usize>;
    async fn get_range_pending_users<F: RichField>(
        &self,
        start: usize,
        end: usize,
    ) -> anyhow::Result<Vec<MerkleProofCore<QHashOut<F>>>>;
}

pub trait RealmPendingUserKey {
    fn realm_pending_user_key(&self) -> String;
}

impl RealmPendingUserKey for super::redis_queue::ProofStoreRedisAsync {
    fn realm_pending_user_key(&self) -> String {
        format!("{}-{}", REALM_PENDING_USER_QUEUE_KEY_PREFIX, self.biz_key())
    }
}

#[async_trait]
impl QPendingUserStoreAsyncImm for super::redis_queue::ProofStoreRedisAsync {
    async fn push_pending_users<F: RichField>(
        &self,
        pending_users: &[MerkleProofCore<QHashOut<F>>],
    ) -> anyhow::Result<()> {
        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");

        for user in pending_users.iter() {
            let user_bytes = bincode::serialize(user).map_err(|e| anyhow::anyhow!(e))?;
            conn.rpush(&key, user_bytes).await?;
        }

        Ok(())
    }

    async fn get_current_pending_user_index(&self) -> anyhow::Result<usize> {
        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.realm_pending_user_key(), "CURRENT_INDEX");
        let index: usize = conn.get(&key).await.unwrap_or(0);
        Ok(index)
    }

    async fn set_current_pending_user_index(&self, index: usize) -> anyhow::Result<()> {
        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.realm_pending_user_key(), "CURRENT_INDEX");
        conn.set(&key, index).await?;
        Ok(())
    }

    async fn get_total_pending_users(&self) -> anyhow::Result<usize> {
        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");
        let length: usize = conn.llen(key).await?;
        Ok(length)
    }

    async fn get_range_pending_users<F: RichField>(
        &self,
        start: usize,
        end: usize,
    ) -> anyhow::Result<Vec<MerkleProofCore<QHashOut<F>>>> {
        let mut conn = self.pool().get().await?;
        let key = format!("{}-{}", self.realm_pending_user_key(), "PENDING_USERS");
        let pending_users_bytes: Vec<Vec<u8>> =
            conn.lrange(&key, start as isize, end as isize).await?;
        let pending_users = pending_users_bytes
            .iter()
            .map(|user| -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
                bincode::deserialize(user).map_err(|e| anyhow::anyhow!(e))
            })
            .collect::<anyhow::Result<Vec<MerkleProofCore<QHashOut<F>>>>>()?;
        Ok(pending_users)
    }
}
