use std::future::Future;
use std::sync::{Arc, atomic::AtomicU64};
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use anyhow::anyhow;
use lazy_static::lazy_static;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_node::coordinator::state::edge::CoordinatorEdgeContext;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use reth_libmdbx::RW;

type StoreReader = KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>>;
type DrainQueue = ProofStoreFred;
type ProofStore = ProofStoreFred;


lazy_static! {
    pub static ref GLOBAL_COORD_EDGE_CTX: Arc<RwLock<Option<CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>>>> =
        Arc::new(RwLock::new(None));
}
pub static LATEST_CHECKPOINT_ID: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

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


// pub fn with_ctx_read_sync<F, R>(f: F) -> anyhow::Result<R>
// where
//     F: FnOnce(&CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>) -> anyhow::Result<R>,
// {
//     let read_guard = GLOBAL_COORD_EDGE_CTX
//         .read()
//         .map_err(|_| anyhow::anyhow!("RwLock poisoned"))?;
//
//     let ctx = read_guard
//         .as_ref()
//         .ok_or_else(|| anyhow::anyhow!("GLOBAL_COORD_EDGE_CTX is not initialized"))?;
//
//     f(ctx)
// }

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

/// (Deprecated)
/// read the global CoordinatorEdgeContext and return the next checkpoint_id.
pub async fn next_checkpoint_id() -> anyhow::Result<u64> {
    let guard = GLOBAL_COORD_EDGE_CTX.read().await;
    let ctx = guard
        .as_ref()
        .ok_or_else(|| anyhow!("GLOBAL_COORD_EDGE_CTX not initialized"))?;

    ctx.get_next_checkpoint_id_async().await
}
