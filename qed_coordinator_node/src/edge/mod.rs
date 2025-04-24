pub mod config;

pub mod context;
pub mod init;
pub mod processor;
pub mod rpc;

use crate::args::CoordinatorEdgeArgs;
use jsonrpsee::server::Server;
use qed_core::utils::debug_timer::DebugTimer;

use crate::edge::init::{init_coordinator_edge, init_tracing};
use crate::edge::rpc::router::build_rpc_module;
use std::net::SocketAddr;
use tracing::info;

pub async fn run_edge(config: CoordinatorEdgeArgs) -> anyhow::Result<()> {
    info!("🚀 Starting coordinator edge node...");
    init_tracing();

    let mut timer = DebugTimer::new("coordinator_edge_node");
    timer.lap("start");

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
