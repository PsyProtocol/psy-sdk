use crate::communicate::get_latest_global_coordinator_status;
use crate::rpc::types::{RealmInfo, RealmRpcRegistry};
use crate::{
    CoordinatorEdgeQueueArgs, COORDINATOR_NOTIFICATIONS_QUEUE_SUFFIX,
    COORDINATOR_WORKER_QUEUE_SUFFIX, COORDINATOR_WORKER_SUFFIX,
};
use anyhow::anyhow;
use dashmap::DashMap;
use dotenvy::var;
use fred::prelude::Pool;
use fred::prelude::*;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use lazy_static::lazy_static;
use once_cell::sync::{Lazy, OnceCell};
use qed_core::data::qhashout::QHashOut;
use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_node::coordinator::state::user_map::get_node_redis_pool;
use qed_node::nimpl::drain_queue_fred::DrainQueueFred;
use qed_node::nimpl::new_fred_pool;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_store::config::store_config::QEDFelt;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::{atomic::AtomicU64, Arc, OnceLock};
use tokio::sync::RwLock;
use tracing::{info, warn};

type StoreReader = KVQArcImmutableStoreWrapper<KVQlibmdbxStore>;
type DrainQueue = ProofStoreFred;
type ProofStore = ProofStoreFred;

lazy_static! {
    //todo! if no write, use once_cell instead
    pub static ref GLOBAL_COORD_EDGE_CTX: Arc<RwLock<Option<CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>>>> =
        Arc::new(RwLock::new(None));
}
pub static LATEST_CHECKPOINT_ID: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static REGISTER_USER_COUNTER: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static REGISTERED_USERS: Lazy<DashMap<QHashOut<QEDFelt>, UserRegisterState>> =
    Lazy::new(DashMap::new);
pub static GLOBAL_DB_PATH: OnceCell<String> = OnceCell::new();

pub static GLOBAL_REDIS_POOL: OnceCell<Arc<Pool>> = OnceCell::new();
pub static GLOBAL_DRAIN_QUEUE: OnceCell<DrainQueueFred> = OnceCell::new();
pub static GLOBAL_REALM_REGISTRY: Lazy<Arc<RwLock<RealmRpcRegistry>>> =
    Lazy::new(|| Arc::new(RwLock::new(RealmRpcRegistry::default())));

pub static GLOBAL_LMDB_STORE: OnceLock<Arc<KVQArcImmutableStoreWrapper<KVQlibmdbxStore>>> =
    OnceLock::new();

pub static GLOBAL_JWT_SECRET: OnceCell<Arc<String>> = OnceCell::new();

//    when user registers, but not write into block, we mark as Registering
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserRegisterState {
    Registering(u64),
    Registered(u64),
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

pub async fn register_realm(name: String, rpc_url: String) -> anyhow::Result<()> {
    let mut registry = GLOBAL_REALM_REGISTRY.write().await;
    if !registry.realms.contains_key(&rpc_url) {
        tracing::info!("✅ Registered new realm: {}", name);
        registry
            .realms
            .insert(rpc_url.clone(), RealmInfo { name, rpc_url });
    } else {
        tracing::info!("ℹ️ Realm already exists: {}", rpc_url);
    }
    Ok(())
}
// pub async fn init_realms_from_env() -> anyhow::Result<()> {
//     let endpoints = var("REALM_RPC_ENDPOINTS")?;
//     let urls: Vec<&str> = endpoints.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
//
//     let mut registry = GLOBAL_REALM_REGISTRY.write().await;
//
//     for (i, url) in urls.iter().enumerate() {
//         if !registry.realms.contains_key(*url) {
//             let name = format!("realm_{}", i + 1);
//             tracing::info!("✅ Loaded realm from .env: {}", name);
//             registry.realms.insert((*url).to_string(), RealmInfo { name, rpc_url: (*url).to_string() });
//         } else {
//             tracing::info!("ℹ️ Realm already exists: {}", url);
//         }
//     }
//
//     Ok(())
// }

pub fn init_global_lmdb_store() -> anyhow::Result<()> {
    if GLOBAL_LMDB_STORE.get().is_some() {
        return Ok(());
    }

    let db_path = get_global_db_path()?;
    let inner_store = KVQlibmdbxStore::new_read(db_path)?;
    let wrapped_store = KVQArcImmutableStoreWrapper::new(inner_store);

    GLOBAL_LMDB_STORE
        .set(Arc::new(wrapped_store))
        .map_err(|_| anyhow!("GLOBAL_LMDB_STORE already initialized"))
}
pub fn get_global_lmdb_store() -> anyhow::Result<Arc<KVQArcImmutableStoreWrapper<KVQlibmdbxStore>>>
{
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
        coordinator_worker_queue_suffix,
        coordinator_notifications_queue_suffix,
        Some(&coordinator_proof_store_key_suffix),
        Some(&coordinator_proof_store_key_suffix),
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

pub fn init_global_jwt_secret() -> anyhow::Result<()> {
    let secret = var("JWT_SECRET_KEY")?;
    GLOBAL_JWT_SECRET
        .set(Arc::new(secret))
        .map_err(|_| anyhow::anyhow!("JWT secret already initialized"))?;
    Ok(())
}
pub fn get_global_jwt_secret() -> Arc<String> {
    GLOBAL_JWT_SECRET
        .get()
        .expect("JWT secret not initialized")
        .clone()
}

pub fn init_global_queue(pool: Pool) {
    let drain_queue = DrainQueueFred::new(pool);

    if GLOBAL_DRAIN_QUEUE.set(drain_queue).is_err() {
        warn!("⚠️ GLOBAL_DRAIN_QUEUE already initialized, skipping re-initialization.");
    }
}
pub async fn check_and_print_latest_coordinator_status() -> anyhow::Result<()> {
    let pool = GLOBAL_REDIS_POOL
        .get()
        .expect("❌ GLOBAL_REDIS_POOL is not initialized");

    let drain_queue = DrainQueueFred::new(pool.as_ref().clone());

    match get_latest_global_coordinator_status(&drain_queue).await? {
        Some(status) => {
            info!(
                "📦 Latest Coordinator Status: checkpoint={}, processor_height={}, timestamp={}",
                status.confirmed_checkpoint_id, status.processor_height, status.timestamp
            );
        }
        None => {
            warn!("⚠️ Coordinator status not yet available.");
        }
    }

    Ok(())
}
