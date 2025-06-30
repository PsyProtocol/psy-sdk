use std::env;
// std
use crate::CoordinatorEdgeArgs;
use anyhow::{anyhow, Context};
use qed_store::store::scylla::ScyllaStore;
use once_cell::sync::Lazy;
use std::future::Future;
use std::sync::{atomic::AtomicU64, Arc, OnceLock};
use tracing::info;

use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_node::coordinator::state::user_map::init_node_redis_pool;
use qed_node::nimpl::drain_queue_redis_async::dq_imm::DrainQueueRedisAsync;
use qed_node::nimpl::{new_fred_pool, new_redis_async_pool};
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::nimpl::proof_store_redis_async::ProofStoreRedisAsync;
use qed_node_common::verifier::get_cached_generic_verifier;

pub type StoreReader = ScyllaStore;
pub type DrainQueue = ProofStoreRedisAsync;
pub type ProofStore = ProofStoreRedisAsync;

pub static LATEST_CHECKPOINT_ID: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

pub struct GlobalCoordinatorEdgeState {
    pub ctx: CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>,
    pub sync_queue: DrainQueueRedisAsync,
    pub store: Arc<StoreReader>,
    pub jwt_secret: Arc<String>,
}
pub static GLOBAL_COORD_EDGE_STATE: OnceLock<GlobalCoordinatorEdgeState> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserRegisterState {
    Registered(u64),
}

pub async fn init_coordinator_edge(config: &CoordinatorEdgeArgs) -> anyhow::Result<()> {
    info!("🚀 Initializing coordinator edge node...");

    // Create ScyllaDB store reader
    info!("🗄️ Using ScyllaDB storage: {}:{}", config.scylla.uri, config.scylla.keyspace);
    let scylla_store = ScyllaStore::new(
        &config.scylla.uri,
        &config.scylla.keyspace,
    ).await?;
    let store_reader = Arc::new(scylla_store);

    let redis_pool = new_redis_async_pool(&config.redis_uri, 8).await?;
    init_node_redis_pool(redis_pool.clone())?;

    let sync_queue = DrainQueueRedisAsync::new(&config.redis_uri).await?;

    let qe_args = &config.queue_args;

    let proof_store = Arc::new(ProofStoreRedisAsync::new2(
        redis_pool.clone(),
        &qe_args.worker_queue_suffix,
        &qe_args.notifications_queue_suffix,
        &qe_args.proof_store_key_suffix,
        &qe_args.proof_store_key_suffix,
    ).await?);

    // init verifier
    let verifier = Arc::new(get_cached_generic_verifier::<_, 2>());

    let edge_config = qed_node::coordinator::state::processor::CoordinatorConfig::get_standard(0);

    // init context
    let ctx = CoordinatorEdgeContext::new(
        edge_config,
        store_reader.clone(),
        Arc::clone(&proof_store),
        Arc::clone(&proof_store),
        verifier,
    )
    .await?;

    let jwt_secret =
        env::var("JWT_SECRET").with_context(|| "JWT_SECRET not found in environment")?;

    let global = GlobalCoordinatorEdgeState {
        ctx,
        sync_queue,
        store: store_reader,
        jwt_secret: Arc::new(jwt_secret),
    };
    GLOBAL_COORD_EDGE_STATE
        .set(global)
        .map_err(|_| anyhow::anyhow!("GLOBAL_COORD_EDGE_STATE already initialized"))?;

    info!("🚀 Coordinator Edge Initialized");

    Ok(())
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

pub fn get_jwt_secret() -> Option<Arc<String>> {
    GLOBAL_COORD_EDGE_STATE
        .get()
        .map(|state| Arc::clone(&state.jwt_secret))
}
