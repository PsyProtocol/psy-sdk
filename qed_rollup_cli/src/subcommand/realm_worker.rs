use qed_realm_node::{QueueConfig, RedisConfig};

use qed_worker::{RealmWorker, Worker, WorkerState};

async fn run_worker(redis_config: RedisConfig, queue_config: QueueConfig) -> anyhow::Result<()> {
    let state = WorkerState::new(
        redis_config.redis_uri,
        redis_config.pool_size.unwrap_or(8),
        queue_config.worker_queue_suffix,
        queue_config.notifications_queue_suffix,
        Some(queue_config.proof_store_key_suffix.as_str()),
        Some(queue_config.proof_store_key_suffix.as_str()),
    )
    .await?;
    let worker = RealmWorker::from(state);
    worker.run().await
}

pub async fn run(redis_config: RedisConfig, queue_config: QueueConfig) -> anyhow::Result<()> {
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
