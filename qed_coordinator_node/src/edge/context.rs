use std::sync::atomic::Ordering;
use crate::{
    CoordinatorEdgeQueueArgs,
};
use anyhow::anyhow;
use dashmap::DashMap;
use fred::prelude::Pool;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use lazy_static::lazy_static;
use once_cell::sync::{Lazy, OnceCell};
use qed_core::data::qhashout::QHashOut;
use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_node::coordinator::state::user_map::get_node_redis_pool;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_store::config::store_config::QEDFelt;
use std::future::Future;
use std::sync::{atomic::AtomicU64, Arc, OnceLock};
use chrono::Utc;
use tokio::sync::RwLock;
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;
use qed_node::nimpl::drain_queue_redis::dq_imm::DrainQueueRedis;
use crate::rpc::types::CheckpointSyncInfo;

pub type StoreReader = KVQArcImmutableStoreWrapper<KVQlibmdbxStore>;
pub type DrainQueue = ProofStoreFred;
pub type ProofStore = ProofStoreFred;


lazy_static! {
    //todo! if no write, use once_cell instead
    pub static ref GLOBAL_COORD_EDGE_CTX: Arc<RwLock<Option<CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>>>> =
        Arc::new(RwLock::new(None));
}
pub static LATEST_CHECKPOINT_ID: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static GLOBAL_DB_PATH: OnceCell<String> = OnceCell::new();

pub static GLOBAL_REDIS_POOL: OnceCell<Arc<Pool>> = OnceCell::new();
pub static GLOBAL_DRAIN_QUEUE: OnceCell<DrainQueueRedis> = OnceCell::new();

pub static GLOBAL_LMDB_STORE: OnceLock<Arc<KVQArcImmutableStoreWrapper<KVQlibmdbxStore>>> =
    OnceLock::new();

// when user registers, but not write into block, we mark as Registering
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserRegisterState {
    Registered(u64),
}

pub async fn with_ctx_read_async<F, Fut, R>(f: F) -> anyhow::Result<R>
where
    F: FnOnce(&CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<R>>,
{
    let read_guard = GLOBAL_COORD_EDGE_CTX.read().await;
    let ctx = read_guard
        .as_ref()
        .ok_or_else(|| anyhow!("GLOBAL_COORD_EDGE_CTX is not initialized"))?;

    f(ctx).await
}

pub async fn with_ctx_write_async<F, Fut, R>(f: F) -> anyhow::Result<R>
where
    F: FnOnce(&mut CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>) -> Fut,
    Fut: Future<Output = anyhow::Result<R>>,
{
    let mut guard = GLOBAL_COORD_EDGE_CTX.write().await;

    let ctx = guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("CoordinatorEdgeContext not initialized"))?;

    f(ctx).await
}

pub fn get_global_db_path() -> anyhow::Result<&'static str> {
    GLOBAL_DB_PATH
        .get()
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow!("GLOBAL_DB_PATH not initialized"))
}

pub fn get_global_lmdb_store() -> anyhow::Result<Arc<KVQArcImmutableStoreWrapper<KVQlibmdbxStore>>> {
    GLOBAL_LMDB_STORE
        .get()
        .cloned()
        .ok_or_else(|| anyhow!("GLOBAL_LMDB_STORE not initialized"))
}
pub async fn with_temp_ctx_read_async<F, Fut, R, C, const D: usize>(
    args: CoordinatorEdgeQueueArgs,
    f: F,
) -> anyhow::Result<R>
where
    F: FnOnce(CoordinatorEdgeContext<KVQArcImmutableStoreWrapper<KVQlibmdbxStore>, ProofStoreFred, ProofStoreFred>) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<R>>,
{
    let read_guard = GLOBAL_COORD_EDGE_CTX.read().await;
    let ctx = read_guard
        .as_ref()
        .ok_or_else(|| anyhow!("GLOBAL_COORD_EDGE_CTX is not initialized"))?;

    let store = get_global_lmdb_store()?;

    let redis_pool = get_node_redis_pool()?;

    let CoordinatorEdgeQueueArgs {
        coordinator_worker_queue_suffix,
        coordinator_notifications_queue_suffix,
        coordinator_proof_store_key_suffix,
        ..
    } = args;
    let proof_store = Arc::new(ProofStoreFred::new2(
        (*redis_pool).clone(),
        &coordinator_worker_queue_suffix,
        &coordinator_notifications_queue_suffix,
        &coordinator_proof_store_key_suffix,
        &coordinator_proof_store_key_suffix,
    ));

    let latest_checkpoint_id = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed);
    let temp_ctx = CoordinatorEdgeContext {
        coordinator_config: ctx.coordinator_config.clone(),
        store_reader: Arc::clone(&store),
        checkpoint_queue: Arc::clone(&proof_store),
        proof_store: Arc::clone(&proof_store),
        proof_verifier: Arc::clone(&ctx.proof_verifier),
        last_chkpnt_id: latest_checkpoint_id,
    };

    f(temp_ctx).await
}

pub fn build_checkpoint_sync_info(
    latest_checkpoint_id: u64,
    checkpoint_sync_info: QEDCheckpointSyncInfoCompact<QEDFelt>,
) -> CheckpointSyncInfo {
    CheckpointSyncInfo {
        latest_checkpoint_id,
        description: None,
        source_coordinator_edge_id: None,
        sync_timestamp: Utc::now().timestamp() as u64,
        compact: checkpoint_sync_info,
    }
}