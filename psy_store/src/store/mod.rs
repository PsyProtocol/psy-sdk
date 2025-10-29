#![cfg(not(target_arch = "wasm32"))]

pub mod backend;
pub mod journal;
pub mod scylla;
pub mod tikv;

use std::sync::Arc;
use auto_impl::auto_impl;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};
pub use kvq_store_lmdbx::KVQlibmdbxStore;
pub use self::backend::{Backend, BackendConfig};
use self::{scylla::ScyllaStore, tikv::TiKVStore};

#[auto_impl(&, Box, Arc)]
pub trait PsyStoreTrait: KVQBinaryStore + KVQBinaryStoreAsync {}

impl PsyStoreTrait for ScyllaStore {}
impl PsyStoreTrait for KVQlibmdbxStore {}
impl PsyStoreTrait for TiKVStore {}

pub type PsyStore = Arc<dyn PsyStoreTrait>;

pub async fn new(backend: &Backend) -> anyhow::Result<PsyStore> {
    from_backend(backend.clone()).await
}

pub async fn from_backend(backend: Backend) -> anyhow::Result<PsyStore> {
    match backend {
        Backend::Scylla(config) => {
            let store = ScyllaStore::new(&config.uri, &config.keyspace).await?;
            Ok(Arc::new(store))
        }
        Backend::Lmdbx(config) => {
            let store = KVQlibmdbxStore::new_write_with_size(&config.lmdbx_path, config.lmdbx_mmap_size_gb)?;
            Ok(Arc::new(store))
        }
        Backend::TiKV(config) => {
            let store = TiKVStore::new(config).await?;
            Ok(Arc::new(store))
        }
    }
}
