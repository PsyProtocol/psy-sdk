use qed_realm_node::RealmEdgeConfig;

pub async fn run(args: RealmEdgeConfig) -> anyhow::Result<()> {
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        result = qed_realm_node::run_realm_edge(args)=> {
            match result {
                Ok(()) => tracing::warn!("Realm edge exit."),
                Err(e) => tracing::error!("Realm edge exit error: {:?}", e),
            }
        }
        _ = ctrl_c => {
            tracing::warn!("Ctrl-C signal received, cleaning up...");
        }
    }
    Ok(())
}
