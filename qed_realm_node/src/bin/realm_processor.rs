use fred::prelude::{Builder, Config, ReconnectPolicy};
use http::Method;
use jsonrpsee::server::Server;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::realm::state::processor::RealmConfig;
use qed_node_common::verifier::get_cached_generic_verifier;
use qed_realm_node::{RealmProcessor, RealmProcessorRpc, RealmProcessorRpcServer, C, D};
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use reth_libmdbx::{Environment, EnvironmentFlags, Mode, SyncMode, RW};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool_size = 8;
    let config = Config::from_url("redis://127.0.0.1:6379")?;
    let pool = Builder::from_config(config)
        .with_connection_config(|config| {
            config.connection_timeout = Duration::from_secs(10);
        })
        // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
        .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
        .build_pool(pool_size)?;
    let realm_qps = ProofStoreFred::new(pool, "rwq1".to_string(), "rnq1".to_string());

    let realm_config = RealmConfig::get_standard(0, 0);

    let flags = EnvironmentFlags {
        no_sub_dir: false,
        mode: Mode::ReadWrite {
            sync_mode: SyncMode::Durable,
        },
        coalesce: true,
        ..Default::default()
    };
    let env = Environment::builder()
        .set_max_dbs(10)
        .set_flags(flags)
        .open(PathBuf::new().join("db").as_path())?;
    let txn = env.begin_rw_txn()?;
    let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>> =
        KVQArcImmutableStoreWrapper::<KVQlibmdbxStore<RW>>::new(KVQlibmdbxStore::new(
            txn.clone(),
            None,
        )?);

    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

    let coordinator_worker_circuits =
        QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);

    let realm_processor = RealmProcessor {
        realm_config,
        realm_qps,
        store_reader,
        proof_verifier,
        coordinator_worker_circuits,
        checkpoint_id: 1,
    };

    let realm_processor_rpc = RealmProcessorRpc {
        processor: Arc::new(tokio::sync::Mutex::new(realm_processor)),
    };

    let rpc_methods = RealmProcessorRpc::into_rpc(realm_processor_rpc);

    let cors = CorsLayer::new()
        .allow_methods([Method::POST])
        .allow_origin(Any)
        .allow_headers([http::header::CONTENT_TYPE]);
    let middleware = ServiceBuilder::new().layer(cors);

    let addr = std::net::SocketAddr::from_str("127.0.0.1:8765")?;
    let server = Server::builder()
        .set_http_middleware(middleware)
        .build(addr)
        .await?;

    let handle = server.start(rpc_methods);

    handle.stopped().await;

    Ok(())
}
