use std::fs;
// std
use crate::{CoordinatorEdgeArgs};
use anyhow::anyhow;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use lazy_static::lazy_static;
use once_cell::sync::{Lazy};
use std::future::Future;
use std::path::Path;
use std::sync::{
    atomic::{AtomicU64},
    Arc, OnceLock,
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_node::coordinator::state::user_map::{init_node_redis_pool};
use qed_node::nimpl::drain_queue_redis::dq_imm::DrainQueueRedis;
use qed_node::nimpl::new_fred_pool;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node_common::verifier::get_cached_generic_verifier;

pub type StoreReader = KVQArcImmutableStoreWrapper<KVQlibmdbxStore>;
pub type DrainQueue = ProofStoreFred;
pub type ProofStore = ProofStoreFred;

pub static LATEST_CHECKPOINT_ID: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

pub struct GlobalCoordinatorEdgeState {
    pub ctx: CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>,
    pub sync_queue: DrainQueueRedis,
    pub store: StoreReader,
}
pub static GLOBAL_COORD_EDGE_STATE: OnceLock<GlobalCoordinatorEdgeState> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserRegisterState {
    Registered(u64),
}

pub async fn init_coordinator_edge(config: &CoordinatorEdgeArgs) -> anyhow::Result<()> {
    info!("🚀 Initializing coordinator edge node...");

    // Check if the coordinator_db_path exists, 12 times, 5 seconds interval
    wait_for_path_exists(&config.coordinator_db_path, 12, 5).await?;

    // initialize lmdb
    let store = KVQlibmdbxStore::new_read(&config.coordinator_db_path)?;
    let store_wrapper = KVQArcImmutableStoreWrapper::new(store);
    let store_reader = Arc::new(store_wrapper.dup());

    let redis_pool = new_fred_pool(&config.coordinator_redis_uri, 8).await?;
    init_node_redis_pool(redis_pool.clone())?;

    let sync_queue = DrainQueueRedis::new(&config.coordinator_redis_uri)?;

    let qe_args = &config.coordinator_edge_queue_args;

    let proof_store = Arc::new(ProofStoreFred::new2(
        redis_pool.clone(),
        &qe_args.coordinator_worker_queue_suffix,
        &qe_args.coordinator_notifications_queue_suffix,
        &qe_args.coordinator_proof_store_key_suffix,
        &qe_args.coordinator_proof_store_key_suffix,
    ));

    // init verifier
    let verifier = Arc::new(get_cached_generic_verifier::<_, 2>());

    let edge_config = qed_node::coordinator::state::processor::CoordinatorConfig::get_standard(0);

    // init context
    let ctx = CoordinatorEdgeContext::new(
        edge_config,
        Arc::clone(&store_reader),
        Arc::clone(&proof_store),
        Arc::clone(&proof_store),
        verifier,
    )
    .await?;

    let global = GlobalCoordinatorEdgeState {
        ctx,
        sync_queue,
        store: store_wrapper,
    };
    GLOBAL_COORD_EDGE_STATE
        .set(global)
        .map_err(|_| anyhow::anyhow!("GLOBAL_COORD_EDGE_STATE already initialized"))?;

    info!("🚀 Coordinator Edge Initialized");

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
            tracing::info!("✅ Found db path: {}", path_ref.display());
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

pub async fn with_temp_ctx_read_async<F, Fut, R, C, const D: usize>(f: F) -> anyhow::Result<R>
where
    F: FnOnce(CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>) -> Fut,
    Fut: Future<Output = anyhow::Result<R>>,
{
    let state = GLOBAL_COORD_EDGE_STATE
        .get()
        .ok_or_else(|| anyhow!("GLOBAL_COORD_EDGE_STATE is not initialized"))?;

    let latest_checkpoint_id = LATEST_CHECKPOINT_ID.load(std::sync::atomic::Ordering::Relaxed);

    let temp_ctx = CoordinatorEdgeContext {
        coordinator_config: state.ctx.coordinator_config.clone(),
        store_reader: Arc::clone(&state.ctx.store_reader),
        checkpoint_queue: Arc::clone(&state.ctx.checkpoint_queue),
        proof_store: Arc::clone(&state.ctx.proof_store),
        proof_verifier: Arc::clone(&state.ctx.proof_verifier),
        last_chkpnt_id: latest_checkpoint_id,
    };

    f(temp_ctx).await
}
