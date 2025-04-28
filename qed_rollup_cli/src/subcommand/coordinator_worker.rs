use qed_coordinator_node::{
    CoordinatorWorkerArgs, COORDINATOR_NOTIFICATIONS_QUEUE_SUFFIX, COORDINATOR_WORKER_QUEUE_SUFFIX,
    COORDINATOR_WORKER_SUFFIX,
};

use qed_worker::{CoordinatorWorker, Worker, WorkerState};

async fn run_worker(args: CoordinatorWorkerArgs) -> anyhow::Result<()> {
    let state= WorkerState::new(
        args.coordinator_redis_uri,
        args.coordinator_pool_size as usize,
        COORDINATOR_WORKER_QUEUE_SUFFIX.to_string(),
        COORDINATOR_NOTIFICATIONS_QUEUE_SUFFIX.to_string(),
        Some(COORDINATOR_WORKER_SUFFIX),
        Some(COORDINATOR_WORKER_SUFFIX),
    ).await?;
    let coordinator_worker = CoordinatorWorker::from(state);
    coordinator_worker.run().await
}

pub async fn run(args: CoordinatorWorkerArgs) -> anyhow::Result<()> {
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        result = run_worker(args) => {
            match result {
                Ok(()) => tracing::warn!("Coordinator worker exit."),
                Err(e) => tracing::error!("Coordinator worker exit error: {:?}", e),
            }
        }
        _ = ctrl_c => {
            tracing::warn!("Ctrl-C signal received, cleaning up...");
        }
    }
    Ok(())
}
