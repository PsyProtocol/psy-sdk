use std::time::Duration;

use fred::{
    prelude::{ClientLike, Config, Pool, ReconnectPolicy},
    types::Builder,
};
use fred::types::{ClusterHash, CustomCommand};

pub mod drain_queue_fred;
pub mod drain_queue_redis;
pub mod proof_store_fred;
pub mod proof_store_redis_async;
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
    {
        //use QED_ROLE env var to set client name for redis-cli tool
        let role = std::env::var("QED_ROLE").unwrap_or_else(|_| "unknown-role".to_string());

        let clients = pool.clients();
        for (index, client) in clients.iter().enumerate() {
            let name = format!("fred-{}-client-{}", role, index);
            // tracing::info!("🔧 Setting CLIENT SETNAME = {} for fred pool client", name);

            // use CustomCommand to send CLIENT SETNAME command
            let command = CustomCommand {
                cmd: "CLIENT".into(),
                cluster_hash: ClusterHash::default(),
                blocking: false,
            };

            client
                .custom::<(), _>(
                    command,
                    vec!["SETNAME", &name]
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to set client name for client {}: {:?}", index, e))?;
        }

        // tracing::info!("✅ All fred clients in pool set with CLIENT SETNAME");

    }
    Ok(pool)
}

/// Create a new bb8-redis connection pool
///
/// # Arguments
///
/// * `redis_url` - Redis URL to connect to
/// * `pool_size` - Number of connections in the pool
pub async fn new_redis_async_pool(redis_url: &str, pool_size: usize) -> anyhow::Result<bb8::Pool<bb8_redis::RedisConnectionManager>> {
    // Create the connection manager
    let manager = bb8_redis::RedisConnectionManager::new(redis_url)?;
    
    // Build the pool with similar configuration to fred pool
    let pool = bb8::Pool::builder()
        .max_size(pool_size as u32)
        .connection_timeout(Duration::from_secs(10))
        .build(manager)
        .await?;
    
    // Optionally add client identification (if supported by redis-rs)
    // This may require getting a connection and executing a command
    if let Ok(role) = std::env::var("QED_ROLE") {
        // Try to set a similar client name if possible
        // Note: bb8-redis doesn't directly expose CLIENT SETNAME
        if let Ok(mut conn) = pool.get().await {
            let _: Result<String, _> = redis::cmd("CLIENT")
                .arg("SETNAME")
                .arg(format!("bb8-{}-pool", role))
                .query_async(&mut *conn)
                .await;
        }
    }
    
    // Log pool creation
    tracing::info!("✅ Created bb8-redis connection pool with size {}", pool_size);
    
    Ok(pool)
}