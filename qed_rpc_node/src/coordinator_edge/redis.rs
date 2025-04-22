use fred::prelude::*;
use std::time::Duration;

pub async fn init_redis_pool(redis_url: &str, pool_size: usize) -> anyhow::Result<Pool> {
    let config = Config::from_url(redis_url)?;
    let pool = Builder::from_config(config)
        .with_connection_config(|config| {
            config.connection_timeout = Duration::from_secs(10);
        })
        .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
        .build_pool(pool_size)?;

    pool.init().await?;
    Ok(pool)
}
