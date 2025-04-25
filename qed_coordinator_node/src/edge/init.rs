use std::sync::Arc;
use crate::args::CoordinatorEdgeArgs;
use crate::edge::context::init_global_ctx_once;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_core::utils::debug_timer::DebugTimer;
use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_node::nimpl::new_fred_pool;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node_common::store::new_lmdbx_env;
use qed_node_common::verifier::get_cached_generic_verifier;
use qed_realm_node::REALM_PROCESSOR_SUFFIX;
use reth_libmdbx::{Environment, EnvironmentFlags, Mode, SyncMode, RO, RW};
use crate::context::init_global_db_path;

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

pub async fn init_coordinator_edge(config: &CoordinatorEdgeArgs) -> anyhow::Result<()> {
    use tracing::info;
    let mut timer = DebugTimer::new("coordinator_edge_node");
    info!("🚀 Initializing coordinator edge node...");

    init_global_db_path(&config.coordinator_db_path)?;
    info!("✅ Initialized global db path");

    let redis_pool = new_fred_pool(&config.coordinator_redis_uri, 8).await?;

    let proof_store = Arc::new(ProofStoreFred::new2(
        redis_pool.clone(),
        "wq1".into(),
        "nq1".into(),
        Some(REALM_PROCESSOR_SUFFIX),
        Some(REALM_PROCESSOR_SUFFIX),
    ));
    timer.lap("redis initialized");
    info!("✅ Initialized Redis pool");

    println!("config: {:#?}", config);
    // initialize lmdb
    std::fs::create_dir_all(&config.coordinator_db_path)?;

    let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore> =
        KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(KVQlibmdbxStore::new_read(&config.coordinator_db_path)?);
    // store_reader.initialize_store()?;
    let store_reader = Arc::new(store_reader.dup());
    timer.lap("lmdb initialized");
    info!("✅ Initialized LMDB");
    // init verifier
    let verifier = Arc::new(get_cached_generic_verifier::<_, 2>());
    info!("✅ Initialized verifier");
    let config = qed_node::coordinator::state::processor::CoordinatorConfig::get_standard(0);
    println!("get coordinator config");
    // init context
    let ctx = CoordinatorEdgeContext::init(
        config,
        Arc::clone(&store_reader),
        Arc::clone(&proof_store),
        Arc::clone(&proof_store),
        verifier,
    )
    .await?;

    init_global_ctx_once(ctx).await?;
    info!("✅ Initialized global context");
    timer.lap("context initialized");

    Ok(())
}
