pub mod communicate;
pub mod context;
pub mod rpc;

use crate::args::CoordinatorEdgeArgs;
use crate::context::{get_jwt_secret, init_coordinator_edge};
use crate::edge::rpc::router::build_rpc_module;
use crate::rpc::jwt::{JwtSecret, ServerLayer};

use hyper::Method;
use jsonrpsee::server::Server;
use std::net::SocketAddr;
use tower_http::cors::{AllowHeaders, Any, CorsLayer};
use tracing::info;

pub async fn run_edge(config: CoordinatorEdgeArgs) -> anyhow::Result<()> {
    info!("🚀 Starting coordinator edge node...");
    info!("✅ Loaded config: {:#?}", config);

    init_coordinator_edge(&config).await?;

    let (rpc_module, handler) = build_rpc_module(config.clone())?;
    handler.spawn_cp_sync_listener().await?;

    let addr: SocketAddr = config.listen_addr.parse()?;

    let jwt_secret = match get_jwt_secret() {
        Some(jwt_secret) => JwtSecret::from_hex(&jwt_secret.as_bytes())?,
        None => {
            return Err(anyhow::anyhow!("JWT secret not found"));
        }
    };

    let cors_opts = CorsLayer::new()
        .allow_methods([
            Method::POST,
            Method::OPTIONS,
        ])
        .allow_origin(Any)
        .allow_headers(Any);
    let cors = tower::ServiceBuilder::new().layer(cors_opts).layer(ServerLayer(jwt_secret));

    let server = Server::builder()
        .set_http_middleware(cors)
        .build(addr)
        .await?;

    info!(
        "🚀 Coordinator Edge RPC server now running on http://{}",
        config.listen_addr
    );
    let handle = server.start(rpc_module);
    handle.stopped().await;
    Ok(())
}
