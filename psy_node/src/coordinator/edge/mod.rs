pub mod error;
pub mod handler;
pub mod jwt;
pub mod rpc;
pub mod types;

use std::{env, net::SocketAddr};

use hyper::Method;
use jsonrpsee::server::Server;
use psy_common::health::HealthLayer;
use psy_store::{queue::ProofStoreRedis, store::PsyStore};
use tower_http::cors::{AllowHeaders, Any, CorsLayer};
use tracing::info;

use self::{
    jwt::{JwtSecret, ServerLayer},
    rpc::CoordinatorEdgeRpcServer,
};
use super::args::CoordinatorEdgeArgs;
use crate::common::jobs::JobSchedulerRpcServer;

pub type StoreReader = PsyStore;
pub type DrainQueue = ProofStoreRedis;
pub type ProofStore = ProofStoreRedis;

pub async fn run_edge(config: CoordinatorEdgeArgs) -> anyhow::Result<()> {
    info!("🚀 Starting coordinator edge node...");
    info!("✅ Loaded config: {:#?}", config);

    let handler = handler::CoordinatorEdgeHandler::new(config.clone()).await?;
    let mut rpc_module = CoordinatorEdgeRpcServer::into_rpc(handler.clone());
    let job_rpc_module = JobSchedulerRpcServer::into_rpc(handler);
    rpc_module.merge(job_rpc_module)?;

    let addr: SocketAddr = config.listen_addr.parse()?;

    let jwt_secret_str = env::var("JWT_SECRET").unwrap_or_else(|_| {
        info!("JWT_SECRET not found in environment, using default for development");
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()
    });
    let jwt_secret = JwtSecret::from_hex(&jwt_secret_str)?;

    let cors_opts = CorsLayer::new()
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(Any);
    let cors = tower::ServiceBuilder::new()
        .layer(HealthLayer)
        .layer(cors_opts)
        .layer(ServerLayer(jwt_secret));

    let server = Server::builder().set_http_middleware(cors).build(addr).await?;

    info!("🚀 Coordinator Edge RPC server now running on http://{}", config.listen_addr);
    let handle = server.start(rpc_module);

    // Return immediately and let the caller handle the server lifecycle
    // The server will continue running in the background
    tokio::spawn(async move {
        handle.stopped().await;
    });

    // Keep the function running
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
