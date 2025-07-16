use qed_node::coordinator::CoordinatorProcessorArgs;

pub async fn run(args: CoordinatorProcessorArgs) -> anyhow::Result<()> {
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        result = qed_node::coordinator::run_processor(args)=> {
            match result {
                Ok(()) => tracing::warn!("Coordinator processor exit."),
                Err(e) => tracing::error!("Coordinator processor exit error: {:?}", e),
            }
        }
        _ = ctrl_c => {
            tracing::warn!("Ctrl-C signal received, cleaning up...");
        }
    }
    Ok(())
}
