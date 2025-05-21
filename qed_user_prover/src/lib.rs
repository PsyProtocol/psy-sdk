pub mod api;
pub mod args;
pub mod common;
pub mod store;

use hyper::Method;
use jsonrpsee::server::Server;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::base_types::hash256::Hash256;
use qed_core::data::qhashout::QHashOut;
use qed_user_cli::rpc::provider::RpcConfig;
use qed_user_cli::subcommand::session::WalletSession;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};

use crate::api::RpcServer;
use crate::api::RpcServerImpl;
use crate::args::ProverArgs;
use crate::common::enc::SimpleZeroPadEncryptionHelper;
use crate::store::UserProverWorkerStore;

pub async fn run_server(args: ProverArgs) -> anyhow::Result<()> {
    let api_key = Hash256::from_hex_string(&args.api_key)?;
    let encryption_helper = SimpleZeroPadEncryptionHelper::new(api_key);

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
    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    let store = Arc::new(Mutex::new(UserProverWorkerStore::new()));
    let wallet_session = Arc::new(RwLock::new(WalletSession::new(&rpc_config, private_key)?));

    let store_rpc = store.clone();
    let rpc_server_impl = RpcServerImpl::new(store_rpc, wallet_session);

    let handle = server.start(rpc_server_impl.into_rpc());

    tokio::spawn(handle.stopped());
    Ok(futures::future::pending::<()>().await)
}
