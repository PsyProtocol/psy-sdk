pub mod context;
pub mod init;
pub mod rpc;
pub mod communicate;

use crate::args::CoordinatorEdgeArgs;
use jsonrpsee::server::Server;
use qed_core::utils::debug_timer::DebugTimer;

use crate::edge::init::{init_coordinator_edge};
use crate::edge::rpc::router::build_rpc_module;
use std::net::SocketAddr;
use tracing::info;

pub async fn run_edge(config: CoordinatorEdgeArgs) -> anyhow::Result<()> {
    info!("🚀 Starting coordinator edge node...");

    let mut timer = DebugTimer::new("coordinator_edge_node");
    timer.lap("start");

    info!("✅ Loaded config: {:#?}", config);

    init_coordinator_edge(&config).await?;
    info!("✅ Initialized coordinator edge node");

    let (rpc_module, handler) = build_rpc_module(config.clone())?;
    handler.spawn_cp_sync_listener().await?;
    info!("✅ Initialized RPC module");

    let addr: SocketAddr = config.coordinator_edge_listen_addr.parse()?;
    let server = Server::builder().build(addr).await?;
    let handle = server.start(rpc_module);
    info!(
        "🚀 CoordinatorEdge RPC server running on http://{}",
        config.coordinator_edge_listen_addr
    );

    handle.stopped().await;
    Ok(())
}
