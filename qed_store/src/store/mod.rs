#![cfg(not(target_arch = "wasm32"))]

pub mod scylla;
pub mod lmdbx;
pub mod backend;

use std::sync::Arc;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync};

use self::scylla::ScyllaStore;
use self::lmdbx::KVQlibmdbxStore;
pub use self::backend::{Backend, BackendConfig};

#[derive(Clone, Debug)]
pub enum QEDStore {
    Scylla(Arc<ScyllaStore>),
    Lmdbx(Arc<KVQlibmdbxStore>),
}

impl QEDStore {
    pub async fn new(backend: &Backend) -> anyhow::Result<Self> {
        Self::from_backend(backend.clone()).await
    }
    
    pub async fn from_backend(backend: Backend) -> anyhow::Result<Self> {
        match backend {
            Backend::Scylla(config) => {
                let store = ScyllaStore::new(&config.uri, &config.keyspace).await?;
                Ok(QEDStore::Scylla(Arc::new(store)))
            }
            Backend::Lmdbx(config) => {
                let store = KVQlibmdbxStore::new_write_with_size(&config.path, config.size_gb)?;
                Ok(QEDStore::Lmdbx(Arc::new(store)))
            }
        }
    }
}

impl KVQBinaryStore for QEDStore {
    fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            QEDStore::Scylla(store) => store.get_exact_if_exists(key),
            QEDStore::Lmdbx(store) => store.get_exact_if_exists(key),
        }
    }

    fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        match self {
            QEDStore::Scylla(store) => store.get_exact(key),
            QEDStore::Lmdbx(store) => store.get_exact(key),
        }
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        match self {
            QEDStore::Scylla(store) => store.get_many_exact(keys),
            QEDStore::Lmdbx(store) => store.get_many_exact(keys),
        }
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            QEDStore::Scylla(store) => store.get_leq(key, fuzzy_bytes),
            QEDStore::Lmdbx(store) => store.get_leq(key, fuzzy_bytes),
        }
    }

    fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>> {
        match self {
            QEDStore::Scylla(store) => store.get_fuzzy_range_leq_kv(key, fuzzy_bytes),
            QEDStore::Lmdbx(store) => store.get_fuzzy_range_leq_kv(key, fuzzy_bytes),
        }
    }

    fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>> {
        match self {
            QEDStore::Scylla(store) => store.get_leq_kv(key, fuzzy_bytes),
            QEDStore::Lmdbx(store) => store.get_leq_kv(key, fuzzy_bytes),
        }
    }

    fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        match self {
            QEDStore::Scylla(store) => store.get_many_leq(keys, fuzzy_bytes),
            QEDStore::Lmdbx(store) => store.get_many_leq(keys, fuzzy_bytes),
        }
    }

    fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>>> {
        match self {
            QEDStore::Scylla(store) => store.get_many_leq_kv(keys, fuzzy_bytes),
            QEDStore::Lmdbx(store) => store.get_many_leq_kv(keys, fuzzy_bytes),
        }
    }

    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        match self {
            QEDStore::Scylla(store) => store.set(key, value),
            QEDStore::Lmdbx(store) => store.set(key, value),
        }
    }

    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        match self {
            QEDStore::Scylla(store) => store.set_ref(key, value),
            QEDStore::Lmdbx(store) => store.set_ref(key, value),
        }
    }

    fn set_many_ref<'a>(&self, items: &[kvq::traits::KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        match self {
            QEDStore::Scylla(store) => store.set_many_ref(items),
            QEDStore::Lmdbx(store) => store.set_many_ref(items),
        }
    }

    fn set_many_vec(&self, items: Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        match self {
            QEDStore::Scylla(store) => store.set_many_vec(items),
            QEDStore::Lmdbx(store) => store.set_many_vec(items),
        }
    }

    fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        match self {
            QEDStore::Scylla(store) => store.delete(key),
            QEDStore::Lmdbx(store) => store.delete(key),
        }
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        match self {
            QEDStore::Scylla(store) => store.delete_many(keys),
            QEDStore::Lmdbx(store) => store.delete_many(keys),
        }
    }

    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        match self {
            QEDStore::Scylla(store) => store.set_many_split_ref(keys, values),
            QEDStore::Lmdbx(store) => store.set_many_split_ref(keys, values),
        }
    }
}

#[async_trait::async_trait]
impl KVQBinaryStoreAsync for QEDStore {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_exact_if_exists(store, key).await,
            QEDStore::Lmdbx(store) => {
                let result = store.get_exact_if_exists(key)?;
                Ok(result)
            }
        }
    }

    async fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_exact(store, key).await,
            QEDStore::Lmdbx(store) => {
                let result = store.get_exact(key)?;
                Ok(result)
            }
        }
    }

    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_many_exact(store, keys).await,
            QEDStore::Lmdbx(store) => {
                let result = store.get_many_exact(keys)?;
                Ok(result)
            }
        }
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_leq(store, key, fuzzy_bytes).await,
            QEDStore::Lmdbx(store) => {
                let result = store.get_leq(key, fuzzy_bytes)?;
                Ok(result)
            }
        }
    }

    async fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_fuzzy_range_leq_kv(store, key, fuzzy_bytes).await,
            QEDStore::Lmdbx(store) => {
                let result = store.get_fuzzy_range_leq_kv(key, fuzzy_bytes)?;
                Ok(result)
            }
        }
    }

    async fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_leq_kv(store, key, fuzzy_bytes).await,
            QEDStore::Lmdbx(store) => {
                let result = store.get_leq_kv(key, fuzzy_bytes)?;
                Ok(result)
            }
        }
    }

    async fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_many_leq(store, keys, fuzzy_bytes).await,
            QEDStore::Lmdbx(store) => {
                let result = store.get_many_leq(keys, fuzzy_bytes)?;
                Ok(result)
            }
        }
    }

    async fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>>> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_many_leq_kv(store, keys, fuzzy_bytes).await,
            QEDStore::Lmdbx(store) => {
                let result = store.get_many_leq_kv(keys, fuzzy_bytes)?;
                Ok(result)
            }
        }
    }

    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::set(store, key, value).await,
            QEDStore::Lmdbx(store) => {
                store.set(key, value)?;
                Ok(())
            }
        }
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::set_ref(store, key, value).await,
            QEDStore::Lmdbx(store) => {
                store.set_ref(key, value)?;
                Ok(())
            }
        }
    }

    async fn set_many_ref<'a>(&self, items: &[kvq::traits::KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::set_many_ref(store, items).await,
            QEDStore::Lmdbx(store) => {
                store.set_many_ref(items)?;
                Ok(())
            }
        }
    }

    async fn set_many_vec(&self, items: Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::set_many_vec(store, items).await,
            QEDStore::Lmdbx(store) => {
                store.set_many_vec(items)?;
                Ok(())
            }
        }
    }

    async fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::delete(store, key).await,
            QEDStore::Lmdbx(store) => {
                let result = store.delete(key)?;
                Ok(result)
            }
        }
    }

    async fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::delete_many(store, keys).await,
            QEDStore::Lmdbx(store) => {
                let result = store.delete_many(keys)?;
                Ok(result)
            }
        }
    }

    async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        match self {
            QEDStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::set_many_split_ref(store, keys, values).await,
            QEDStore::Lmdbx(store) => {
                store.set_many_split_ref(keys, values)?;
                Ok(())
            }
        }
    }
}
