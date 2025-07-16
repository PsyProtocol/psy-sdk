use std::env;
// std
use crate::coordinator::args::CoordinatorEdgeArgs;
use anyhow::{anyhow, Context};
use qed_store::store::{QEDStore, Backend};
use once_cell::sync::Lazy;
use std::future::Future;
use std::sync::{atomic::AtomicU64, Arc, OnceLock};
use tracing::info;

use crate::coordinator::state::edge::CoordinatorEdgeContext;
use qed_store::queue::drain_queue_redis_async::dq_imm::DrainQueueRedisAsync;
use qed_store::queue::{new_fred_pool, new_redis_async_pool};
use qed_store::queue::proof_store_fred::ProofStoreFred;
use qed_store::queue::proof_store_redis_async::ProofStoreRedisAsync;
use crate::common::verifier::get_cached_generic_verifier;

pub type StoreReader = QEDStore;
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

pub async fn init_coordinator_edge(config: &CoordinatorEdgeArgs) -> anyhow::Result<()> {
    info!("🚀 Initializing coordinator edge node...");

    // Create QED store reader from backend configuration
    info!("🗄️ Initializing storage backend...");
    let qed_store = QEDStore::from_backend(config.backend.to_backend()).await?;
    let store_reader = Arc::new(qed_store);

    let redis_pool = new_redis_async_pool(&config.redis_uri, 8).await?;

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

    let edge_config = crate::coordinator::state::processor::CoordinatorConfig::get_standard(0);

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
