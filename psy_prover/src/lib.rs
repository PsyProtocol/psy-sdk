pub mod local;
pub mod session;
pub mod wallet;

#[cfg(not(target_arch = "wasm32"))]
pub mod health;

// WASM exports
#[cfg(target_arch = "wasm32")]
pub mod wasm;

use psy_config::PSY_NETWORK_MAGIC;

use crate::local::native::prove_proxy::ProveProxyServerProvider;

pub async fn run_server(args: crate::local::args::ProverArgs) -> anyhow::Result<()> {
    use std::{net::SocketAddr, sync::Arc};

    use hyper::Method;
    use jsonrpsee::server::Server;
    use parking_lot::{Mutex, RwLock};
    use psy_common::data::base_types::hash256::Hash256;
    use psy_provider::provider::NetworkConfig;
    use tower_http::cors::{Any, CorsLayer};

    use crate::{
        health::HealthLayer,
        local::{
            common::enc::SimpleZeroPadEncryptionHelper,
            native::{RpcServer, RpcServerImpl},
            UserProverWorkerStore,
        },
        session::WalletSession,
    };

    let api_key = Hash256::from_hex_string(&args.api_key)?;
    let _encryption_helper = SimpleZeroPadEncryptionHelper::new(api_key);

    let cors_opts = CorsLayer::new()
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(Any);
    let cors = tower::ServiceBuilder::new().layer(HealthLayer).layer(cors_opts);

    let server_addr: SocketAddr = args.listen_addr.parse()?;
    tracing::info!("Starting user prover server at {}", server_addr);

    let server = Server::builder().set_http_middleware(cors).build(server_addr).await?;

    let psy_config = psy_config::PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?;

    // let store = Arc::new(Mutex::new(UserProverWorkerStore::new()));
    let wallet_session = Arc::new(RwLock::new(WalletSession::new(&rpc_config).await?));
    let rpc_server_impl = RpcServerImpl::new(wallet_session);
    let handle = server.start(rpc_server_impl.into_rpc());
    handle.stopped().await;
    Ok(())
}

pub async fn run_prove_proxy_server(args: crate::local::args::ProveProxyArgs) -> anyhow::Result<()> {
    use std::net::SocketAddr;

    use hyper::Method;
    use jsonrpsee::server::Server;
    use psy_provider::provider::NetworkConfig;
    use tower_http::cors::{Any, CorsLayer};

    use crate::{health::HealthLayer, local::native::prove_proxy::ProveProxyRpcServer};

    let psy_config = psy_config::PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?;
    let prove_proxy = ProveProxyServerProvider::new_with_config(rpc_config.clone(), PSY_NETWORK_MAGIC).await?;
    let cors_opts = CorsLayer::new()
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(Any);
    let cors = tower::ServiceBuilder::new().layer(HealthLayer).layer(cors_opts);
    let server_addr: SocketAddr = args.listen_addr.parse()?;
    tracing::info!("Starting prove proxy server at {}", server_addr);
    let server = Server::builder().set_http_middleware(cors).build(server_addr).await?;

    let handle = server.start(prove_proxy.into_rpc());
    handle.stopped().await;
    Ok(())
}
