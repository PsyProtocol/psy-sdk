use jsonrpsee::{server::Server};
use qed_core::{
    utils::debug_timer::DebugTimer,
};
use qed_coordinator_edge::coordinator_edge::config::AppConfig;

use std::net::SocketAddr;
use tracing::info;
use qed_coordinator_edge::coordinator_edge::init::{init_coordinator_edge, init_tracing};
use qed_coordinator_edge::coordinator_edge::rpc::router::build_rpc_module;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    info!("🚀 Starting coordinator edge node...");
    init_tracing();

    let mut timer = DebugTimer::new("coordinator_edge_node");
    timer.lap("start");

    let config = AppConfig::from_env();
    info!("✅ Loaded config: {:#?}", config);

    init_coordinator_edge(&config).await?;
    info!("✅ Initialized coordinator edge node");

    let (rpc_module, handler) = build_rpc_module(&config.redis_url)?;
    handler.spawn_cp_sync_listener().await?;
    handler.spawn_realm_job_listener().await?;
    info!("✅ Initialized RPC module");

    let addr: SocketAddr = format!("0.0.0.0:{}", config.coordinator_edge_port).parse()?;
    let server = Server::builder().build(addr).await?;
    let handle = server.start(rpc_module);
    info!(
        "🚀 CoordinatorEdge RPC server running on http://0.0.0.0:{}",
        config.coordinator_edge_port
    );

    handle.stopped().await;
    Ok(())
}
