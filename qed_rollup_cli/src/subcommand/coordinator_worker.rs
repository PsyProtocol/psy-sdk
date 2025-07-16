use qed_node::coordinator::CoordinatorWorkerArgs;
use tracing::info;

use qed_node::worker::{CoordinatorWorker, Worker, WorkerState};

async fn run_worker(args: CoordinatorWorkerArgs) -> anyhow::Result<()> {
    let state = WorkerState::new(
        args.redis_uri,
        args.redis_pool_size as usize,
        &args.queue_args.worker_queue_suffix,
        &args.queue_args.notifications_queue_suffix,
        &args.queue_args.proof_store_key_suffix,
        &args.queue_args.proof_store_key_suffix,
    )
    .await?;
    let coordinator_worker = CoordinatorWorker::from(state);
    coordinator_worker.run().await
}

pub async fn run(args: CoordinatorWorkerArgs) -> anyhow::Result<()> {
    info!("Coordinator worker starting...");
    info!("Coordinator worker args: {:?}", args);
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
