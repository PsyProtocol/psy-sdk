pub mod api;
pub mod store;

#[cfg(not(target_arch = "wasm32"))]
pub mod args;

#[cfg(not(target_arch = "wasm32"))]
pub mod common;

// Only export what's necessary for WASM
#[cfg(target_arch = "wasm32")]
pub use api::WasmRpcServer;

// For non-WASM targets
#[cfg(not(target_arch = "wasm32"))]
pub use store::UserProverWorkerStore;

#[cfg(not(target_arch = "wasm32"))]
pub async fn run_server(args: args::ProverArgs) -> anyhow::Result<()> {
    use hyper::Method;
    use jsonrpsee::server::Server;
    use qed_core::data::base_types::hash256::Hash256;
    use qed_user_cli::rpc::provider::RpcConfig;
    use qed_user_cli::session::WalletSession;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex, RwLock};
    use tower_http::cors::{Any, CorsLayer};

    use crate::api::{RpcServer, RpcServerImpl};
    use crate::common::enc::SimpleZeroPadEncryptionHelper;

    let api_key = Hash256::from_hex_string(&args.api_key)?;
    let _encryption_helper = SimpleZeroPadEncryptionHelper::new(api_key);

    let cors = CorsLayer::new()
        // Allow `POST` when accessing the resource
        .allow_methods([Method::POST])
        // Allow requests from any origin
        .allow_origin(Any)
        .allow_headers([hyper::header::CONTENT_TYPE]);
    let middleware = tower::ServiceBuilder::new().layer(cors);

    let server_addr: SocketAddr = args.listen_addr.parse()?;

    let server = Server::builder()
        .set_http_middleware(middleware)
        .build(server_addr)
        .await?;

    let rpc_config: RpcConfig = serde_json::from_str(&std::fs::read_to_string(args.rpc_config)?)?;

    let store = Arc::new(Mutex::new(UserProverWorkerStore::new()));
    let wallet_session = Arc::new(RwLock::new(WalletSession::new(&rpc_config)?));

    let store_rpc = store.clone();
    let rpc_server_impl = RpcServerImpl::new(store_rpc, wallet_session);

    let handle = server.start(rpc_server_impl.into_rpc());

    tokio::spawn(handle.stopped());
    Ok(futures::future::pending::<()>().await)
}
