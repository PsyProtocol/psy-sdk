pub mod communicate;
pub mod context;
pub mod rpc;


use tower::Service;
use std::iter::once;
use jsonrpsee::server::Server;
use std::net::SocketAddr;
use std::sync::Arc;
use axum::{Router, routing::post, extract::State, response::IntoResponse, http::Request};
use jsonrpsee::RpcModule;
use tower::{service_fn, ServiceBuilder};
use tracing::info;
use crate::args::CoordinatorEdgeArgs;
use crate::context::{get_jwt_secret, init_coordinator_edge};
use crate::edge::rpc::router::build_rpc_module;
use crate::rpc::jwt::{JwtSecret, ServerLayer};
use hyper::body::Body;
use axum::http::Method;
use hyper::body::Bytes;
use headers::HeaderValue;
use jsonrpsee::rpc_params;
use std::time::Duration;
use http::header::AUTHORIZATION;


use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::ws_client::WsClientBuilder;
use headers::HeaderName;

pub async fn run_edge(config: CoordinatorEdgeArgs) -> anyhow::Result<()> {
    info!("🚀 Starting coordinator edge node...");
    info!("✅ Loaded config: {:#?}", config);

    init_coordinator_edge(&config).await?;

    let (rpc_module, handler) = build_rpc_module(config.clone())?;
    handler.spawn_cp_sync_listener().await?;

    let addr: SocketAddr = config.coordinator_edge_listen_addr.parse()?;

    let jwt_secret = match get_jwt_secret() {
        Some(jwt_secret) => {
            JwtSecret::from_hex(&jwt_secret.as_bytes())?
        }
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
