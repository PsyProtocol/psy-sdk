use qed_node::realm::RealmEdgeConfig;

pub async fn run(args: RealmEdgeConfig) -> anyhow::Result<()> {
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        result = qed_node::realm::run_realm_edge_v2(args)=> {
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
