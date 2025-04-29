use once_cell::sync::OnceCell;
use std::sync::Arc;
use anyhow::anyhow;
use fred::clients::Pool;
use fred::prelude::KeysInterface;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_store::config::store_config::QEDFelt;
use hex::encode;
use bincode;
use kvq::traits::KVQSerializable;

pub const USER_ID_KEY_PREFIX: &str = "qed:reg:user_id:";
pub const PUBKEY_KEY_PREFIX: &str = "qed:reg:pubkey:";

pub async fn save_user_mapping_to_redis(
    redis_pool: &Pool,
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
    redis_pool
        .set::<(), _, _>(user_key, pubkey_info_bytes, None, None, false)
        .await?;

    redis_pool
        .set::<(), _, _>(pubkey_key, user_id.to_string(), None, None, false)
        .await?;

    Ok(())
}

pub async fn get_pubkey_info_by_user_id(
    redis_pool: &Pool,
    user_id: u64,
) -> anyhow::Result<Option<ZKPublicKeyInfo<QEDFelt>>> {
    let user_key = format!("{}{}", USER_ID_KEY_PREFIX, user_id);
    let result: Option<Vec<u8>> = redis_pool.get(user_key).await.ok();

    if let Some(bytes) = result {
        let info = bincode::deserialize::<ZKPublicKeyInfo<QEDFelt>>(&bytes)?;
        Ok(Some(info))
    } else {
        Ok(None)
    }
}

pub async fn get_user_id_by_pubkey(
    redis_pool: &Pool,
    pubkey_hex: &str,
) -> anyhow::Result<Option<u64>> {
    let pubkey_key = format!("{}{}", PUBKEY_KEY_PREFIX, pubkey_hex);
    let result: Option<String> = redis_pool.get(pubkey_key).await.ok();

    Ok(result.and_then(|s| s.parse::<u64>().ok()))
}



static GLOBAL_NODE_REDIS_POOL: OnceCell<Arc<Pool>> = OnceCell::new();

pub fn init_node_redis_pool(pool: Pool) -> anyhow::Result<()> {
    GLOBAL_NODE_REDIS_POOL
        .set(Arc::new(pool))
        .map_err(|_| anyhow!("GLOBAL_NODE_REDIS_POOL already initialized"))
}

pub fn get_node_redis_pool() -> anyhow::Result<Arc<Pool>> {
    GLOBAL_NODE_REDIS_POOL
        .get()
        .cloned()
        .ok_or_else(|| anyhow!("GLOBAL_NODE_REDIS_POOL not initialized"))
}