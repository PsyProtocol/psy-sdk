use std::future::Future;
use std::path::Path;
use std::sync::{Arc, atomic::AtomicU64};
use tokio::sync::RwLock;
use once_cell::sync::{Lazy, OnceCell};
use anyhow::anyhow;
use dashmap::DashMap;
use lazy_static::lazy_static;
use log::info;
use plonky2::hash::hash_types::RichField;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_store::config::store_config::QEDFelt;
use qed_store::node::coordinator::store_traits::QEDCoordinatorStoreReaderAsync;
use reth_libmdbx::{Environment, EnvironmentFlags, Geometry, Mode, RO, RW};

type StoreReader = KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RO>>;
type DrainQueue = ProofStoreFred;
type ProofStore = ProofStoreFred;


lazy_static! {
    //todo! if no write, use once_cell instead
    pub static ref GLOBAL_COORD_EDGE_CTX: Arc<RwLock<Option<CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>>>> =
        Arc::new(RwLock::new(None));
}
pub static LATEST_CHECKPOINT_ID: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static REGISTER_USER_COUNTER: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
pub static REGISTERED_USERS: Lazy<DashMap<QHashOut<QEDFelt>, u64>> = Lazy::new(DashMap::new);
pub static GLOBAL_ENV: OnceCell<Arc<Environment>> = OnceCell::new();



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



pub fn init_global_env(db_path: &str) -> anyhow::Result<()> {
    let env = Environment::builder()
        .set_max_dbs(10)
        .set_flags(EnvironmentFlags {
            no_sub_dir: false,
            mode: Mode::ReadOnly,
            coalesce: true,
            ..Default::default()
        })
        .open(Path::new(db_path))?;
    GLOBAL_ENV.set(Arc::new(env)).map_err(|_| anyhow!("GLOBAL_ENV already set"))
}

pub fn get_global_env() -> anyhow::Result<Arc<Environment>> {
    GLOBAL_ENV.get().cloned().ok_or_else(|| anyhow!("GLOBAL_ENV is not initialized"))
}



pub async fn with_temp_ctx_read_async<F, Fut, R, C, const D: usize>(
    f: F,
) -> anyhow::Result<R>
where
    F: FnOnce(CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<R>>,
{
    let read_guard = GLOBAL_COORD_EDGE_CTX.read().await;
    let ctx = read_guard
        .as_ref()
        .ok_or_else(|| anyhow!("GLOBAL_COORD_EDGE_CTX is not initialized"))?;

    let env = get_global_env()?;
    let txn = env.begin_ro_txn()?;
    let new_store: KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RO>> = KVQArcImmutableStoreWrapper::<KVQlibmdbxStore<RO>>::new(KVQlibmdbxStore::new(txn.clone(), None)?);

    let temp_ctx = CoordinatorEdgeContext {
        coordinator_config: ctx.coordinator_config.clone(),
        store_reader: Arc::new(new_store),
        checkpoint_queue: Arc::clone(&ctx.checkpoint_queue),
        proof_store: Arc::clone(&ctx.proof_store),
        proof_verifier: Arc::clone(&ctx.proof_verifier),
        last_chkpnt_id: ctx.last_chkpnt_id,
    };

    f(temp_ctx).await
}


/// (Deprecated)
/// read the global CoordinatorEdgeContext and return the next checkpoint_id.
pub async fn next_checkpoint_id() -> anyhow::Result<u64> {
    let guard = GLOBAL_COORD_EDGE_CTX.read().await;
    let ctx = guard
        .as_ref()
        .ok_or_else(|| anyhow!("GLOBAL_COORD_EDGE_CTX not initialized"))?;

    ctx.get_next_checkpoint_id_async().await
}


// pub async fn with_store_reader_async<F, R>(
//     env: Arc<Environment>,
//     db: Option<&str>,
//     f: impl for<'a> FnOnce(Arc<dyn QEDCoordinatorStoreReaderAsync<F> + Send + Sync>) -> R,
// ) -> R::Output
// where
//     F: RichField,
//     R: std::future::Future,
// {
//     let txn = env.begin_ro_txn().expect("failed to create txn");
//     let store = KVQlibmdbxStore::new(txn, db).expect("failed to create store");
//     let arc: Arc<dyn QEDCoordinatorStoreReaderAsync<F> + Send + Sync> = Arc::new(store);
//     f(arc).await
// }

