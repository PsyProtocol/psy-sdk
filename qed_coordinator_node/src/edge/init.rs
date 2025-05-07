use crate::args::CoordinatorEdgeArgs;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_core::utils::debug_timer::DebugTimer;
use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_node::nimpl::new_fred_pool;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node_common::verifier::get_cached_generic_verifier;
use std::sync::Arc;
use anyhow::anyhow;
use fred::clients::Pool;
use tracing::warn;
use qed_node::coordinator::state::user_map::init_node_redis_pool;
use qed_realm_node::REALM_PROCESSOR_SUFFIX;
use qed_node::nimpl::drain_queue_fred::DrainQueueFred;
use crate::context::{get_global_db_path, DrainQueue, ProofStore, StoreReader, GLOBAL_COORD_EDGE_CTX, GLOBAL_DB_PATH, GLOBAL_DRAIN_QUEUE, GLOBAL_LMDB_STORE};

pub async fn init_coordinator_edge(config: &CoordinatorEdgeArgs) -> anyhow::Result<()> {
    use tracing::info;
    let mut timer = DebugTimer::new("coordinator_edge_node");
    info!("🚀 Initializing coordinator edge node...");

    init_global_db_path(&config.coordinator_db_path)?;
    info!("✅ Initialized global db path");


    let redis_pool = new_fred_pool(&config.coordinator_redis_uri, 8).await?;
    init_node_redis_pool(redis_pool.clone())?;
    info!("✅ Initialized Redis pool");

    init_global_queue(redis_pool.clone());
    info!("✅ Initialized global drain queue");

    let proof_store = Arc::new(ProofStoreFred::new2(
        redis_pool.clone(),
        &config.coordinator_edge_queue_args.coordinator_worker_queue_suffix,
        &config.coordinator_edge_queue_args.coordinator_notifications_queue_suffix,
        &config.coordinator_edge_queue_args.coordinator_proof_store_key_suffix,
        &config.coordinator_edge_queue_args.coordinator_proof_store_key_suffix,
    ));
    timer.lap("redis initialized");
    info!("✅ Initialized Redis pool");

    // initialize lmdb
    std::fs::create_dir_all(&config.coordinator_db_path)?;
    init_global_lmdb_store()?;
    info!("✅ Initialized global LMDB store");

    let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore> =
        KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(KVQlibmdbxStore::new_read(
            &config.coordinator_db_path,
        )?);
    let store_reader = Arc::new(store_reader.dup());
    timer.lap("lmdb initialized");
    info!("✅ Initialized LMDB");

    // init verifier
    let verifier = Arc::new(get_cached_generic_verifier::<_, 2>());
    info!("✅ Initialized verifier");

    let config = qed_node::coordinator::state::processor::CoordinatorConfig::get_standard(0);
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


pub async fn init_global_ctx_once(
    ctx: CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>,
) -> anyhow::Result<()> {
    let mut guard = GLOBAL_COORD_EDGE_CTX.write().await;

    if guard.is_some() {
        anyhow::bail!("GLOBAL_COORD_EDGE_CTX has already been initialized");
    }

    *guard = Some(ctx);
    Ok(())
}


/// Initialize global DB path (can only be called once)
pub fn init_global_db_path<P: Into<String>>(path: P) -> anyhow::Result<()> {
    GLOBAL_DB_PATH.set(path.into()).map_err(|_| anyhow!("GLOBAL_DB_PATH already set"))
}


pub fn init_global_lmdb_store() -> anyhow::Result<()> {
    if GLOBAL_LMDB_STORE.get().is_some() {
        return Ok(());
    }

    let db_path = get_global_db_path()?;
    let inner_store = KVQlibmdbxStore::new_read(db_path)?;
    let wrapped_store = KVQArcImmutableStoreWrapper::new(inner_store);

    GLOBAL_LMDB_STORE.set(Arc::new(wrapped_store))
        .map_err(|_| anyhow!("GLOBAL_LMDB_STORE already initialized"))
}

pub fn init_global_queue(pool: Pool) {
    let drain_queue = DrainQueueFred::new(pool);

    if GLOBAL_DRAIN_QUEUE.set(drain_queue).is_err() {
        warn!("⚠️ GLOBAL_DRAIN_QUEUE already initialized, skipping re-initialization.");
    }
}