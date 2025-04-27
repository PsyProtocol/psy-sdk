use qed_coordinator_node::CoordinatorWorkerArgs;

pub async fn run(args: CoordinatorWorkerArgs) -> anyhow::Result<()> {
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        result = qed_coordinator_node::run_worker(args)=> {
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
