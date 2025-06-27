use anyhow::anyhow;
use bb8::Pool;
use once_cell::sync::OnceCell;
use std::sync::Arc;

use bb8_redis::{
    redis::{AsyncCommands, RedisResult},
    RedisConnectionManager,
};
use bincode;
use fred::prelude::KeysInterface;
use hex::encode;
use kvq::traits::KVQSerializable;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_store::config::store_config::QEDFelt;

pub const USER_ID_KEY_PREFIX: &str = "qed:reg:user_id:";
pub const PUBKEY_KEY_PREFIX: &str = "qed:reg:pubkey:";

pub async fn save_user_mapping_to_redis(
    redis_pool: &Pool<RedisConnectionManager>,
    user_id: u64,
    pubkey_info: &ZKPublicKeyInfo<QEDFelt>,
) -> anyhow::Result<()> {
    // 1. public_key_param into hex
    let public_key_hex = pubkey_info.public_key_hex_string();

    // 2. ZKPublicKeyInfo -> bytes
    let pubkey_info_bytes = pubkey_info.to_bytes()?;

    // 3. key
    let user_key = format!("{}{}", USER_ID_KEY_PREFIX, user_id);
    let pubkey_key = format!("{}{}", PUBKEY_KEY_PREFIX, public_key_hex);

    // 4.save to redis
    let mut conn = redis_pool.get().await?;
    // user_id -> ZKPublicKeyInfo
    conn.set::<_, _, ()>(&user_key, &pubkey_info_bytes).await?;

    // pubkey_hex -> user_id
    conn.set::<_, _, ()>(&pubkey_key, user_id.to_string())
        .await?;

    Ok(())
}

pub async fn get_pubkey_info_by_user_id(
    redis_pool: &Pool<RedisConnectionManager>,
    user_id: u64,
) -> anyhow::Result<Option<ZKPublicKeyInfo<QEDFelt>>> {
    let user_key = format!("{}{}", USER_ID_KEY_PREFIX, user_id);

    let mut conn = redis_pool.get().await?;
    let result: Option<Vec<u8>> = conn.get(&user_key).await?;

    if let Some(bytes) = result {
        let info = bincode::deserialize::<ZKPublicKeyInfo<QEDFelt>>(&bytes)?;
        Ok(Some(info))
    } else {
        Ok(None)
    }
}

pub async fn get_user_id_by_pubkey(
    redis_pool: &Pool<RedisConnectionManager>,
    pubkey_hex: &str,
) -> anyhow::Result<Option<u64>> {
    let pubkey_key = format!("{}{}", PUBKEY_KEY_PREFIX, pubkey_hex);
    let mut conn = redis_pool.get().await?;
    let a = conn.get(&pubkey_key).await?;
    let result: Option<String> = conn.get(&pubkey_key).await?;

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
