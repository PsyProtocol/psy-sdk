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
use tracing::{info, warn};
use qed_node::coordinator::state::user_map::init_node_redis_pool;
use qed_node::nimpl::drain_queue_redis::dq_imm::DrainQueueRedis;
use crate::context::{DrainQueue, ProofStore, StoreReader, GLOBAL_COORD_EDGE_CTX, GLOBAL_DRAIN_QUEUE, GLOBAL_LMDB_STORE};

pub async fn init_coordinator_edge(config: &CoordinatorEdgeArgs) -> anyhow::Result<()> {
    use tracing::info;
    let mut timer = DebugTimer::new("coordinator_edge_node");
    info!("🚀 Initializing coordinator edge node...");

    // initialize lmdb
    init_global_lmdb_store(&config.coordinator_db_path)?;
    info!("✅ Initialized global LMDB store");

    let redis_pool = new_fred_pool(&config.coordinator_redis_uri, 8).await?;
    init_node_redis_pool(redis_pool.clone())?;
    info!("✅ Initialized Redis pool");

    init_global_queue(&config.coordinator_redis_uri);
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

    init_global_ctx(ctx).await?;
    info!("✅ Initialized global context");
    timer.lap("context initialized");

    Ok(())
}


pub async fn init_global_ctx(
    ctx: CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>,
) -> anyhow::Result<()> {
    let mut guard = GLOBAL_COORD_EDGE_CTX.write().await;

    if guard.is_some() {
        anyhow::bail!("GLOBAL_COORD_EDGE_CTX has already been initialized");
    }

    *guard = Some(ctx);
    Ok(())
}

pub fn init_global_lmdb_store<P: AsRef<str>>(db_path: P) -> anyhow::Result<()> {
    if GLOBAL_LMDB_STORE.get().is_some() {
        return Ok(());
    }

    let store = KVQlibmdbxStore::new_read(db_path.as_ref())?;
    let wrapped_store = KVQArcImmutableStoreWrapper::new(store);

    GLOBAL_LMDB_STORE
        .set(Arc::new(wrapped_store))
        .map_err(|_| anyhow!("GLOBAL_LMDB_STORE already initialized"))
}

pub fn init_global_queue(redis_url : &str) {
    let drain_queue = match DrainQueueRedis::new(redis_url) {
        Ok(dq) => {
            info!("✅ Initialized global drain queue");
            dq
        }
        Err(e) => {
            warn!("❌ Failed to initialize global drain queue: {}", e);
            return;
        }
    };

    if GLOBAL_DRAIN_QUEUE.set(drain_queue).is_err() {
        warn!("⚠️ GLOBAL_DRAIN_QUEUE already initialized, skipping re-initialization.");
    }
}