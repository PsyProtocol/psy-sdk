use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;
use qed_node::realm::RealmNodeConfig;

pub async fn run(args: RealmNodeConfig) -> anyhow::Result<()> {
    let ctrl_c = tokio::signal::ctrl_c();
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("Failed to install SIGTERM handler");
    tokio::select! {
        result = qed_node::realm::run_realm_processor(args)=> {
            match result {
                Ok(()) => tracing::warn!("Realm processor exit."),
                Err(e) => tracing::error!("Realm processor exit error: {:?}", e),
            }
        }
        _ = ctrl_c => {
            tracing::warn!("Ctrl-C signal received, cleaning up...");
        }
        _ = sigterm.recv() => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        }
    }
    Ok(())
}
