pub mod communicate;
pub mod context;
pub mod rpc;

use jsonrpsee::server::Server;
use std::net::SocketAddr;
use tracing::info;

use crate::args::CoordinatorEdgeArgs;
use crate::context::init_coordinator_edge;
use crate::edge::rpc::router::build_rpc_module;
pub async fn run_edge(config: CoordinatorEdgeArgs) -> anyhow::Result<()> {
    info!("🚀 Starting coordinator edge node...");
    info!("✅ Loaded config: {:#?}", config);

    init_coordinator_edge(&config).await?;

    let (rpc_module, handler) = build_rpc_module(config.clone())?;
    handler.spawn_cp_sync_listener().await?;

    let addr: SocketAddr = config.listen_addr.parse()?;
    let server = Server::builder().build(addr).await?;
    let handle = server.start(rpc_module);
    info!(
        "🚀 Coordinator Edge RPC server now running on http://{}",
        config.listen_addr
    );

    handle.stopped().await;
    Ok(())
}
