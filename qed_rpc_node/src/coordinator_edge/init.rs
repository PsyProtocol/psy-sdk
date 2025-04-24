use std::path::Path;
use std::sync::Arc;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_coordinator_node::worker::COORDINATOR_WORKER_SUFFIX;
use qed_core::utils::debug_timer::DebugTimer;
use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_node::nimpl::new_fred_pool;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node_common::verifier::get_cached_generic_verifier;
use reth_libmdbx::{Environment, EnvironmentFlags, Mode, SyncMode, RO, RW};
use crate::coordinator_edge::config::AppConfig;
use crate::coordinator_edge::context::init_global_ctx_once;

pub fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_timer(fmt::time::ChronoUtc::default())
        .with_target(true) // display target
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
}


pub async fn init_coordinator_edge(config: &AppConfig) -> anyhow::Result<()> {
    use tracing::info;
    let mut timer = DebugTimer::new("coordinator_edge_node");
    info!("🚀 Initializing coordinator edge node...");
    let redis_pool = new_fred_pool(&config.redis_url, 8).await?;

    let proof_store = Arc::new(ProofStoreFred::new2(
        redis_pool.clone(),
        "wq1".into(),
        "nq1".into(),
        Some(COORDINATOR_WORKER_SUFFIX),
        Some(COORDINATOR_WORKER_SUFFIX),
    ));
    timer.lap("redis initialized");
    info!("✅ Initialized Redis pool");
    // initialize lmdb
    std::fs::create_dir_all(&config.qed_db_path)?;
    let env = Environment::builder()
        .set_max_dbs(10)
        .set_flags(EnvironmentFlags {
            no_sub_dir: false,
            mode: Mode::ReadOnly,
            coalesce: true,
            ..Default::default()
        })
        .open(Path::new(&config.qed_db_path))?;

    let txn = env.begin_ro_txn()?;
    let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RO>> = KVQArcImmutableStoreWrapper::<KVQlibmdbxStore<RO>>::new(KVQlibmdbxStore::new(txn.clone(), None)?);
    // store_reader.initialize_store()?;
    let store_reader = Arc::new(store_reader.dup());
    timer.lap("lmdb initialized");
    info!("✅ Initialized LMDB");
    // init verifier
    let verifier = Arc::new(get_cached_generic_verifier::<_, 2>());
    info!("✅ Initialized verifier");
    let config =         qed_node::coordinator::state::processor::CoordinatorConfig::get_standard(0);
    println!("get coordinator config");
    // init context
    let ctx = CoordinatorEdgeContext::init(
        config,
        Arc::clone(&store_reader),
        Arc::clone(&proof_store),
        Arc::clone(&proof_store),
        verifier,
    ).await?;

    init_global_ctx_once(ctx).await?;
    info!("✅ Initialized global context");
    timer.lap("context initialized");

    Ok(())
}