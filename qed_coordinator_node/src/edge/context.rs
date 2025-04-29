use crate::{
    COORDINATOR_NOTIFICATIONS_QUEUE_SUFFIX, COORDINATOR_WORKER_QUEUE_SUFFIX,
    COORDINATOR_WORKER_SUFFIX,
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
use qed_node::nimpl::new_fred_pool;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_store::config::store_config::QEDFelt;
use std::future::Future;
use std::sync::OnceLock;
use std::sync::{atomic::AtomicU64, Arc};
use tokio::sync::RwLock;

type StoreReader = KVQArcImmutableStoreWrapper<KVQlibmdbxStore>;
type DrainQueue = ProofStoreFred;
type ProofStore = ProofStoreFred;

lazy_static! {
    //todo! if no write, use once_cell instead
    pub static ref GLOBAL_COORD_EDGE_CTX: Arc<RwLock<Option<CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>>>> =
        Arc::new(RwLock::new(None));
}

pub static GLOBAL_LMDB_STORE: OnceLock<Arc<KVQArcImmutableStoreWrapper<KVQlibmdbxStore>>> = OnceLock::new();

pub static LATEST_CHECKPOINT_ID: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static REGISTER_USER_COUNTER: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static REGISTERED_USERS: Lazy<DashMap<QHashOut<QEDFelt>, u64>> = Lazy::new(DashMap::new);
pub static GLOBAL_DB_PATH: OnceCell<String> = OnceCell::new();

pub static GLOBAL_REDIS_POOL: OnceCell<Arc<Pool>> = OnceCell::new();

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

/// Initialize global DB path (can only be called once)
pub fn init_global_db_path<P: Into<String>>(path: P) -> anyhow::Result<()> {
    GLOBAL_DB_PATH
        .set(path.into())
        .map_err(|_| anyhow!("GLOBAL_DB_PATH already set"))
}
pub fn get_global_db_path() -> anyhow::Result<&'static str> {
    GLOBAL_DB_PATH
        .get()
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow!("GLOBAL_DB_PATH not initialized"))
}

pub async fn init_global_redis_pool_from_url(
    redis_url: &str,
    pool_size: usize,
) -> anyhow::Result<()> {
    let pool = new_fred_pool(redis_url, pool_size).await?;
    GLOBAL_REDIS_POOL
        .set(Arc::new(pool))
        .map_err(|_| anyhow!("GLOBAL_REDIS_POOL already initialized"))
}

pub fn init_global_redis_pool(redis_pool: Pool) -> anyhow::Result<()> {
    GLOBAL_REDIS_POOL
        .set(Arc::new(redis_pool))
        .map_err(|_| anyhow!("GLOBAL_REDIS_POOL already initialized"))
}
pub fn get_global_redis_pool() -> anyhow::Result<Arc<Pool>> {
    GLOBAL_REDIS_POOL
        .get()
        .cloned()
        .ok_or_else(|| anyhow!("GLOBAL_REDIS_POOL not initialized"))
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
 pub fn get_global_lmdb_store() -> anyhow::Result<Arc<KVQArcImmutableStoreWrapper<KVQlibmdbxStore>>> {
     GLOBAL_LMDB_STORE
         .get()
         .cloned()
         .ok_or_else(|| anyhow!("GLOBAL_LMDB_STORE not initialized"))
 }

pub async fn with_temp_ctx_read_async<F, Fut, R, C, const D: usize>(f: F) -> anyhow::Result<R>
where
    F: FnOnce(
        CoordinatorEdgeContext<
            KVQArcImmutableStoreWrapper<KVQlibmdbxStore>,
            ProofStoreFred,
            ProofStoreFred,
        >,
    ) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<R>>,
{
    let read_guard = GLOBAL_COORD_EDGE_CTX.read().await;
    let ctx = read_guard
        .as_ref()
        .ok_or_else(|| anyhow!("GLOBAL_COORD_EDGE_CTX is not initialized"))?;

    let store = get_global_lmdb_store()?;

    let redis_pool = get_global_redis_pool()?;

    let proof_store = Arc::new(ProofStoreFred::new2(
        (*redis_pool).clone(),
        COORDINATOR_WORKER_QUEUE_SUFFIX.into(),
        COORDINATOR_NOTIFICATIONS_QUEUE_SUFFIX.into(),
        Some(COORDINATOR_WORKER_SUFFIX),
        Some(COORDINATOR_WORKER_SUFFIX),
    ));

    let temp_ctx = CoordinatorEdgeContext {
        coordinator_config: ctx.coordinator_config.clone(),
        store_reader: Arc::clone(&store),
        checkpoint_queue: Arc::clone(&proof_store),
        proof_store: Arc::clone(&proof_store),
        proof_verifier: Arc::clone(&ctx.proof_verifier),
        last_chkpnt_id: ctx.last_chkpnt_id,
    };

    f(temp_ctx).await
}
