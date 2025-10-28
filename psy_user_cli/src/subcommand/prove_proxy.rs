use psy_prover::local::args::ProveProxyArgs;

pub async fn run(args: ProveProxyArgs) -> anyhow::Result<()> {
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        result = psy_prover::run_prove_proxy_server(args) => {
            match result {
                Ok(()) => tracing::warn!("Prove proxy server exit."),
                Err(e) => tracing::error!("Prove proxy server exit error: {:?}", e),
            }
        }
        _ = ctrl_c => {
            tracing::warn!("Ctrl-C signal received, cleaning up...");
        }
    }
    Ok(())
}