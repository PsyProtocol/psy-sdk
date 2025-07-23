pub mod dpn;
pub mod ups;
pub mod local;
pub mod wallet;

// Session module is only available for native builds
#[cfg(not(target_arch = "wasm32"))]
pub mod session;

// Re-export the appropriate session module based on target
#[cfg(not(target_arch = "wasm32"))]
pub use session::WalletSession;
#[cfg(not(target_arch = "wasm32"))]
pub use session::WalletKeyPair;

#[cfg(target_arch = "wasm32")]
pub use local::wasm::wallet_session::WalletSession;
#[cfg(target_arch = "wasm32")]
pub use local::types::WalletKeyPair;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main() {
    // Initialize panic hook for better error messages in browser console
    console_error_panic_hook::set_once();

    // Initialize wasm-logger for log crate compatibility
    wasm_logger::init(wasm_logger::Config::default());

    // Initialize tracing subscriber for WASM
    wasm_tracing::set_as_global_default();

    // Log initialization success
    tracing::info!("WASM module initialized successfully with tracing support");
}

// Optional manual initialization function
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn init_logging() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    wasm_tracing::set_as_global_default();
    tracing::info!("Logging initialized manually");
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn run_server(args: crate::local::args::ProverArgs) -> anyhow::Result<()> {
    use hyper::Method;
    use jsonrpsee::server::Server;
    use crate::local::UserProverWorkerStore;
    use qed_core::data::base_types::hash256::Hash256;
    use crate::local::provider::RpcConfig;
    use crate::session::WalletSession;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex, RwLock};
    use tower_http::cors::{Any, CorsLayer};

    use crate::local::native::{RpcServer, RpcServerImpl};
    use crate::local::common::enc::SimpleZeroPadEncryptionHelper;

    let api_key = Hash256::from_hex_string(&args.api_key)?;
    let _encryption_helper = SimpleZeroPadEncryptionHelper::new(api_key);

    let cors_opts = CorsLayer::new()
        .allow_methods([
            Method::POST,
            Method::OPTIONS,
        ])
        .allow_origin(Any)
        .allow_headers(Any);
    let cors = tower::ServiceBuilder::new().layer(cors_opts);

    let server_addr: SocketAddr = args.listen_addr.parse()?;
    tracing::info!("Starting user prover server at {}", server_addr);

    let server = Server::builder()
        .set_http_middleware(cors)
        .build(server_addr)
        .await?;

    let config_str = std::fs::read_to_string(&args.rpc_config)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
    let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;

    let store = Arc::new(Mutex::new(UserProverWorkerStore::new()));
    let wallet_session = Arc::new(RwLock::new(WalletSession::new(&rpc_config)?));

    let store_rpc = store.clone();
    let rpc_server_impl = RpcServerImpl::new(store_rpc, wallet_session);

    let handle = server.start(rpc_server_impl.into_rpc());

    tokio::spawn(handle.stopped());
    Ok(futures::future::pending::<()>().await)
}
