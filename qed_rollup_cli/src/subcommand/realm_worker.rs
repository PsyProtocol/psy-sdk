use qed_node::realm::{QueueConfig, RedisConfig};

use qed_node::worker::run_realm_scheduler_worker;
use tracing::info;

pub async fn run(
    redis_config: RedisConfig,
    queue_config: QueueConfig,
    edge_url: String,
) -> anyhow::Result<()> {
    info!("Realm worker starting...");
    info!("Realm worker args: {:?}, {:?}", redis_config, queue_config);
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        result = run_realm_scheduler_worker(edge_url) => {
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
