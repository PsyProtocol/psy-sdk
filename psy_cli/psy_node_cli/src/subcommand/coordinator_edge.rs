use psy_node::coordinator::CoordinatorEdgeArgs;

pub async fn run(args: CoordinatorEdgeArgs) -> anyhow::Result<()> {
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        result = psy_node::coordinator::edge::run_edge(args) => {
            match result {
                Ok(()) => tracing::warn!("Coordinator edge exit."),
                Err(e) => tracing::error!("Coordinator edge exit error: {:?}", e),
            }
        }
        _ = ctrl_c => {
            tracing::warn!("Ctrl-C signal received, cleaning up...");
        }
    }
    Ok(())
}
