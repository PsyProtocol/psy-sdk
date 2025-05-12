use std::time::Duration;

use fred::{
    prelude::{ClientLike, Config, Pool, ReconnectPolicy},
    types::Builder,
};

pub mod drain_queue_fred;
pub mod drain_queue_redis;
pub mod proof_store_fred;
pub mod proof_store_redis;
pub mod worker_queue_ampq;
pub mod worker_queue_rabbit;
pub mod worker_queue_rabbit_stream;
pub mod worker_queue_redis;

/// Create a new Redis connection pool
///
/// # Arguments
///
/// * `redis_url` - Redis URL to connect tobool
/// * `pool_size` - Number of connections in the pool
pub async fn new_fred_pool(redis_url: &str, pool_size: usize) -> anyhow::Result<Pool> {
    let config = Config::from_url(redis_url)?;
    let pool = Builder::from_config(config)
        .with_connection_config(|config| {
            config.connection_timeout = Duration::from_secs(10);
        })
        // Use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
        .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
        .build_pool(pool_size)?;

    pool.init().await?;
    Ok(pool)
}
