use once_cell::sync::OnceCell;
use qed_crypto::hash::traits::qhashable::QFieldHashable;
use std::sync::Arc;
use anyhow::anyhow;
use fred::clients::Pool;
use fred::prelude::KeysInterface;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_store::config::store_config::QEDFelt;
use hex::encode;
use bincode;
use kvq::traits::KVQSerializable;
use qed_store::config::store_config::QEDHasher;

pub const USER_ID_KEY_PREFIX: &str = "qed:reg:user_id:";
pub const PUBKEY_KEY_PREFIX: &str = "qed:reg:pubkey:";

pub async fn save_user_mapping_to_redis(
    redis_pool: &Pool,
    user_id: u64,
    pubkey_info: &ZKPublicKeyInfo<QEDFelt>,
) -> anyhow::Result<()> {
    let public_key = pubkey_info.qfhash::<QEDHasher>();

    let user_key = format!("{}{}", USER_ID_KEY_PREFIX, user_id);
    let public_key = format!("{}{}", PUBKEY_KEY_PREFIX, public_key);

    redis_pool
        .set::<(), _, _>(user_key.clone(), public_key.clone(), None, None, false)
        .await?;

    redis_pool
        .set::<(), _, _>(public_key, user_id.to_string(), None, None, false)
        .await?;

    Ok(())
}

pub async fn get_user_id_by_pubkey(
    redis_pool: &Pool,
    public_key: &str,
) -> anyhow::Result<Option<u64>> {
    let pubkey_key = format!("{}{}", PUBKEY_KEY_PREFIX, public_key);
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
