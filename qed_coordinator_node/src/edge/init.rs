use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use tokio::time::sleep;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use tracing::{info, warn};

use crate::args::CoordinatorEdgeArgs;
use crate::context::{
    DrainQueue, ProofStore, StoreReader, GLOBAL_COORD_EDGE_CTX, GLOBAL_DRAIN_QUEUE,
    GLOBAL_LMDB_STORE,
};

use qed_core::utils::debug_timer::DebugTimer;

use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_node::coordinator::state::user_map::init_node_redis_pool;
use qed_node::nimpl::drain_queue_redis::dq_imm::DrainQueueRedis;
use qed_node::nimpl::{new_fred_pool, proof_store_fred::ProofStoreFred};

use qed_node_common::verifier::get_cached_generic_verifier;

pub async fn init_coordinator_edge(config: &CoordinatorEdgeArgs) -> anyhow::Result<()> {
    info!("🚀 Initializing coordinator edge node...");

    // Check if the coordinator_db_path exists
    wait_for_path_exists(&config.coordinator_db_path, 12, 5).await?;

    // initialize lmdb
    init_global_lmdb_store(&config.coordinator_db_path)?;

    let redis_pool = new_fred_pool(&config.coordinator_redis_uri, 8).await?;
    init_node_redis_pool(redis_pool.clone())?;

    init_global_queue(&config.coordinator_redis_uri)?;

    let qe_args = &config.coordinator_edge_queue_args;

    let proof_store = Arc::new(ProofStoreFred::new2(
        redis_pool.clone(),
        &qe_args.coordinator_worker_queue_suffix,
        &qe_args.coordinator_notifications_queue_suffix,
        &qe_args.coordinator_proof_store_key_suffix,
        &qe_args.coordinator_proof_store_key_suffix,
    ));

    let store = KVQlibmdbxStore::new_read(&config.coordinator_db_path)?;
    let store_reader = Arc::new(KVQArcImmutableStoreWrapper::new(store).dup());
    // init verifier
    let verifier = Arc::new(get_cached_generic_verifier::<_, 2>());

    let edge_config = qed_node::coordinator::state::processor::CoordinatorConfig::get_standard(0);

    // init context
    let ctx = CoordinatorEdgeContext::init(
        edge_config,
        Arc::clone(&store_reader),
        Arc::clone(&proof_store),
        Arc::clone(&proof_store),
        verifier,
    )
    .await?;

    init_global_ctx(ctx).await?;
    info!("🚀 Coordinator Edge Initialized");

    Ok(())
}

pub async fn init_global_ctx(
    ctx: CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>,
) -> anyhow::Result<()> {
    let mut guard = GLOBAL_COORD_EDGE_CTX.write().await;

    if guard.is_some() {
        return Err(anyhow!(
            "GLOBAL_COORD_EDGE_CTX has already been initialized"
        ));
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

pub fn init_global_queue(redis_url: &str) -> anyhow::Result<()> {
    let drain_queue = DrainQueueRedis::new(redis_url)
        .map_err(|e| anyhow!("Failed to create DrainQueueRedis: {}", e))?;

    GLOBAL_DRAIN_QUEUE
        .set(drain_queue)
        .map_err(|_| anyhow!("GLOBAL_DRAIN_QUEUE already initialized"))?;

    Ok(())
}

pub async fn wait_for_path_exists<P: AsRef<Path>>(
    path: P,
    max_attempts: usize,
    interval_secs: u64,
) -> anyhow::Result<()> {
    let path_ref = path.as_ref();

    for attempt in 0..max_attempts {
        if fs::metadata(path_ref).is_ok() {
            // tracing::info!("✅ Found path: {}", path_ref.display());
            return Ok(());
        } else {
            tracing::warn!(
                "⏳ Path not found: {} (attempt {}/{}) — waiting {}s...",
                path_ref.display(),
                attempt,
                max_attempts,
                interval_secs
            );
            sleep(Duration::from_secs(interval_secs)).await;
        }
    }

    Err(anyhow!(
        "❌ Path not found after {} attempts: {}",
        max_attempts,
        path_ref.display()
    ))
}
