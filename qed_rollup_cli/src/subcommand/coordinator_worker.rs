use qed_node::{coordinator::CoordinatorWorkerArgs, worker::run_coordinator_scheduler_worker};
use tracing::{error, info};

async fn run_worker(args: CoordinatorWorkerArgs) -> anyhow::Result<()> {
    run_coordinator_scheduler_worker(args.edge_url).await?;
    error!("Coordinator worker exit.");
    Ok(())
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
