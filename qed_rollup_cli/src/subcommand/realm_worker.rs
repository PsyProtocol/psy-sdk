use qed_node::realm::{QueueConfig, RedisConfig};

use qed_node::worker::{RealmWorker, Worker, WorkerState};
use tracing::info;

async fn run_worker(redis_config: RedisConfig, queue_config: QueueConfig) -> anyhow::Result<()> {
    let state = WorkerState::new(
        redis_config.redis_uri,
        redis_config.pool_size.unwrap_or(10),
        queue_config.queue_biz_key.clone(),
    )
    .await?;
    let worker = RealmWorker::from(state);
    worker.run().await
}

pub async fn run(redis_config: RedisConfig, queue_config: QueueConfig) -> anyhow::Result<()> {
    info!("Realm worker starting...");
    info!("Realm worker args: {:?}, {:?}", redis_config, queue_config);
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        result = run_worker(redis_config, queue_config) => {
            match result {
                Ok(()) => tracing::warn!("Realm worker exit."),
                Err(e) => tracing::error!("Realm worker exit error: {:?}", e),
            }
        }
        _ = ctrl_c => {
            tracing::warn!("Ctrl-C signal received, cleaning up...");
        }
    }
    Ok(())
}
