use qed_coordinator_node::CoordinatorWorkerArgs;

use qed_worker::{CoordinatorWorker, Worker, WorkerState};

async fn run_worker(args: CoordinatorWorkerArgs) -> anyhow::Result<()> {
    let state= WorkerState::new(
        args.coordinator_redis_uri,
        args.coordinator_pool_size as usize,
        args.coordinator_processor_queue_args
            .coordinator_worker_queue_suffix
            .clone(),
        args.coordinator_processor_queue_args
            .coordinator_notifications_queue_suffix
            .clone(),
        Some(
            args.coordinator_processor_queue_args
                .coordinator_proof_store_key_suffix
                .as_str(),
        ),
        Some(
            args.coordinator_processor_queue_args
                .coordinator_proof_store_key_suffix
                .as_str(),
        ),
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
