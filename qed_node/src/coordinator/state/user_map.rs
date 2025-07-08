use anyhow::anyhow;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use fred::prelude::KeysInterface;
use hex::encode;
use kvq::traits::KVQSerializable;
use once_cell::sync::OnceCell;
use std::sync::Arc;

use bb8_redis::redis::AsyncCommands;
use bincode;
use qed_crypto::hash::traits::qhashable::QFieldHashable;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_store::config::store_config::QEDFelt;
use qed_store::config::store_config::QEDHasher;

pub const USER_ID_KEY_PREFIX: &str = "qed:reg:user_id:";
pub const PUBKEY_KEY_PREFIX: &str = "qed:reg:pubkey:";

pub async fn save_user_mapping_to_redis(
    redis_pool: &Pool<RedisConnectionManager>,
    user_id: u64,
    pubkey_info: &ZKPublicKeyInfo<QEDFelt>,
) -> anyhow::Result<()> {
    let public_key = pubkey_info.qfhash::<QEDHasher>();

    let user_key = format!("{}{}", USER_ID_KEY_PREFIX, user_id);
    let public_key = format!("{}{}", PUBKEY_KEY_PREFIX, public_key);

    let mut conn = redis_pool.get().await?;
    conn.set::<_, _, ()>(&user_key, &public_key).await?;
    conn.set::<_, _, ()>(&public_key, user_id.to_string())
        .await?;

    Ok(())
}

pub async fn get_user_id_by_pubkey(
    redis_pool: &Pool<RedisConnectionManager>,
    public_key: &str,
) -> anyhow::Result<Option<u64>> {
    let public_key = format!("{}{}", PUBKEY_KEY_PREFIX, public_key);
    let mut conn = redis_pool.get().await?;
    let result: Option<String> = conn.get(&public_key).await?;

    Ok(result.and_then(|s| s.parse::<u64>().ok()))
}

static GLOBAL_NODE_REDIS_POOL: OnceCell<Arc<Pool<RedisConnectionManager>>> = OnceCell::new();

pub fn init_node_redis_pool(pool: Pool<RedisConnectionManager>) -> anyhow::Result<()> {
    GLOBAL_NODE_REDIS_POOL
        .set(Arc::new(pool))
        .map_err(|_| anyhow!("GLOBAL_NODE_REDIS_POOL already initialized"))
}

pub fn get_node_redis_pool() -> anyhow::Result<Arc<Pool<RedisConnectionManager>>> {
    GLOBAL_NODE_REDIS_POOL
        .get()
        .cloned()
        .ok_or_else(|| anyhow!("GLOBAL_NODE_REDIS_POOL not initialized"))
}
