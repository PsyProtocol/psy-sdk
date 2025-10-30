pub mod realm;
pub mod utils;

use std::{net::SocketAddr, sync::Arc};

use clap::{Args, Parser, ValueEnum};
use hyper::Method;
use jsonrpsee::server::Server;
use qed_prover::health::HealthLayer;
use qed_store::store::{journal::JournalStore, BackendConfig, QEDStore};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

use crate::subcommand::store::realm::RealmStoreRpcServer;

#[derive(Debug, Clone, PartialEq, ValueEnum, Deserialize, Serialize, Parser)]
pub enum DBMode {
    #[clap(name = "coordinator")]
    Coordinator,
    #[clap(name = "realm")]
    Realm,
}

#[derive(Clone, Debug, Args)]
pub struct StoreConfig {
    #[command(flatten)]
    pub backend: BackendConfig,

    #[arg(long, default_value = "0")]
    pub checkpoint_id: u64,

    #[arg(long, default_value = "0")]
    pub user_id: u64,

    #[arg(long, default_value = "127.0.0.1:11111")]
    pub listen_addr: String,

    #[arg(long, default_value = "realm")]
    pub db_mode: DBMode,
}

#[derive(Clone)]
pub struct StoreProvider {
    pub store: Arc<JournalStore<QEDStore>>,
}

pub async fn run_realm_store_server(args: StoreConfig) -> anyhow::Result<()> {
    let store = JournalStore::new(QEDStore::new(&args.backend.to_backend()).await?);
    let store_provider = StoreProvider { store: Arc::new(store) };
    let cors_opts = CorsLayer::new()
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(Any);
    let cors = tower::ServiceBuilder::new().layer(HealthLayer).layer(cors_opts);
    let server_addr: SocketAddr = args.listen_addr.parse()?;
    tracing::info!("Starting realm store server at {}", server_addr);
    let server = Server::builder().set_http_middleware(cors).build(server_addr).await?;

    let handle = server.start(store_provider.into_rpc());
    handle.stopped().await;
    Ok(())
}

pub async fn run(config: StoreConfig) -> anyhow::Result<()> {
    run_realm_store_server(config).await?;
    Ok(())
}
