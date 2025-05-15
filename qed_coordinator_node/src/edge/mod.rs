pub mod communicate;
pub mod context;
pub mod rpc;

use crate::args::CoordinatorEdgeArgs;
use crate::context::{get_jwt_secret, init_coordinator_edge};
use crate::edge::rpc::router::build_rpc_module;
use crate::rpc::jwt::{JwtSecret, ServerLayer};

use jsonrpsee::server::Server;
use std::net::SocketAddr;
use tracing::info;

pub async fn run_edge(config: CoordinatorEdgeArgs) -> anyhow::Result<()> {
    info!("🚀 Starting coordinator edge node...");
    info!("✅ Loaded config: {:#?}", config);

    init_coordinator_edge(&config).await?;

    let (rpc_module, handler) = build_rpc_module(config.clone())?;
    handler.spawn_cp_sync_listener().await?;

    let addr: SocketAddr = config.coordinator_edge_listen_addr.parse()?;

    let jwt_secret = match get_jwt_secret() {
        Some(jwt_secret) => JwtSecret::from_hex(&jwt_secret.as_bytes())?,
        None => {
            return Err(anyhow::anyhow!("JWT secret not found"));
        }
    };

    let service_builder = tower::ServiceBuilder::new().layer(ServerLayer(jwt_secret));

    let server = Server::builder()
        .set_http_middleware(service_builder)
        .build(addr)
        .await?;

    info!(
        "🚀 Coordinator Edge RPC server now running on http://{}",
        config.coordinator_edge_listen_addr
    );
    let handle = server.start(rpc_module);
    handle.stopped().await;
    Ok(())
}
