#![cfg(not(target_arch = "wasm32"))]

pub mod backend;
pub mod journal;
pub mod lmdbx;
pub mod scylla;
pub mod tikv;

use std::sync::Arc;

use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};

pub use self::backend::{Backend, BackendConfig};
use self::{lmdbx::KVQlibmdbxStore, scylla::ScyllaStore, tikv::TiKVStore};

#[derive(Clone, Debug)]
pub enum PsyStore {
    Scylla(Arc<ScyllaStore>),
    Lmdbx(Arc<KVQlibmdbxStore>),
    TiKV(Arc<TiKVStore>),
}

impl PsyStore {
    pub async fn new(backend: &Backend) -> anyhow::Result<Self> {
        Self::from_backend(backend.clone()).await
    }

    pub async fn from_backend(backend: Backend) -> anyhow::Result<Self> {
        match backend {
            Backend::Scylla(config) => {
                let store = ScyllaStore::new(&config.uri, &config.keyspace).await?;
                Ok(PsyStore::Scylla(Arc::new(store)))
            }
            Backend::Lmdbx(config) => {
                let store = KVQlibmdbxStore::new_write_with_size(&config.lmdbx_path, config.lmdbx_mmap_size_gb)?;
                Ok(PsyStore::Lmdbx(Arc::new(store)))
            }
            Backend::TiKV(config) => {
                let store = TiKVStore::new(config).await?;
                Ok(PsyStore::TiKV(Arc::new(store)))
            }
        }
    }
}

impl KVQBinaryStore for PsyStore {
    fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            PsyStore::Scylla(store) => store.get_exact_if_exists(key),
            PsyStore::Lmdbx(store) => store.get_exact_if_exists(key),
            PsyStore::TiKV(store) => store.get_exact_if_exists(key),
        }
    }

    fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        match self {
            PsyStore::Scylla(store) => store.get_exact(key),
            PsyStore::Lmdbx(store) => store.get_exact(key),
            PsyStore::TiKV(store) => store.get_exact(key),
        }
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        match self {
            PsyStore::Scylla(store) => store.get_many_exact(keys),
            PsyStore::Lmdbx(store) => store.get_many_exact(keys),
            PsyStore::TiKV(store) => store.get_many_exact(keys),
        }
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            PsyStore::Scylla(store) => store.get_leq(key, fuzzy_bytes),
            PsyStore::Lmdbx(store) => store.get_leq(key, fuzzy_bytes),
            PsyStore::TiKV(store) => store.get_leq(key, fuzzy_bytes),
        }
    }

    fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>> {
        match self {
            PsyStore::Scylla(store) => store.get_fuzzy_range_leq_kv(key, fuzzy_bytes),
            PsyStore::Lmdbx(store) => store.get_fuzzy_range_leq_kv(key, fuzzy_bytes),
            PsyStore::TiKV(store) => store.get_fuzzy_range_leq_kv(key, fuzzy_bytes),
        }
    }

    fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>> {
        match self {
            PsyStore::Scylla(store) => store.get_leq_kv(key, fuzzy_bytes),
            PsyStore::Lmdbx(store) => store.get_leq_kv(key, fuzzy_bytes),
            PsyStore::TiKV(store) => store.get_leq_kv(key, fuzzy_bytes),
        }
    }

    fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        match self {
            PsyStore::Scylla(store) => store.get_many_leq(keys, fuzzy_bytes),
            PsyStore::Lmdbx(store) => store.get_many_leq(keys, fuzzy_bytes),
            PsyStore::TiKV(store) => store.get_many_leq(keys, fuzzy_bytes),
        }
    }

    fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>>> {
        match self {
            PsyStore::Scylla(store) => store.get_many_leq_kv(keys, fuzzy_bytes),
            PsyStore::Lmdbx(store) => store.get_many_leq_kv(keys, fuzzy_bytes),
            PsyStore::TiKV(store) => store.get_many_leq_kv(keys, fuzzy_bytes),
        }
    }

    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => store.set(key, value),
            PsyStore::Lmdbx(store) => store.set(key, value),
            PsyStore::TiKV(store) => store.set(key, value),
        }
    }

    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => store.set_ref(key, value),
            PsyStore::Lmdbx(store) => store.set_ref(key, value),
            PsyStore::TiKV(store) => store.set_ref(key, value),
        }
    }

    fn set_many_ref<'a>(&self, items: &[kvq::traits::KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => store.set_many_ref(items),
            PsyStore::Lmdbx(store) => store.set_many_ref(items),
            PsyStore::TiKV(store) => store.set_many_ref(items),
        }
    }

    fn set_many_vec(&self, items: Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => store.set_many_vec(items),
            PsyStore::Lmdbx(store) => store.set_many_vec(items),
            PsyStore::TiKV(store) => store.set_many_vec(items),
        }
    }

    fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        match self {
            PsyStore::Scylla(store) => store.delete(key),
            PsyStore::Lmdbx(store) => store.delete(key),
            PsyStore::TiKV(store) => store.delete(key),
        }
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        match self {
            PsyStore::Scylla(store) => store.delete_many(keys),
            PsyStore::Lmdbx(store) => store.delete_many(keys),
            PsyStore::TiKV(store) => store.delete_many(keys),
        }
    }

    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => store.set_many_split_ref(keys, values),
            PsyStore::Lmdbx(store) => store.set_many_split_ref(keys, values),
            PsyStore::TiKV(store) => store.set_many_split_ref(keys, values),
        }
    }

    fn set_and_delete_many(&self, keys_to_set: &[KVQPair<&Vec<u8>, &Vec<u8>>], keys_to_delete: &[Vec<u8>]) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => store.set_and_delete_many(keys_to_set, keys_to_delete),
            PsyStore::Lmdbx(store) => store.set_and_delete_many(keys_to_set, keys_to_delete),
            PsyStore::TiKV(store) => store.set_and_delete_many(keys_to_set, keys_to_delete),
        }
    }
}

#[async_trait::async_trait]
impl KVQBinaryStoreAsync for PsyStore {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_exact_if_exists(store, key).await,
            PsyStore::Lmdbx(store) => {
                let result = store.get_exact_if_exists(key)?;
                Ok(result)
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::get_exact_if_exists(store, key).await?;
                Ok(result)
            }
        }
    }

    async fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_exact(store, key).await,
            PsyStore::Lmdbx(store) => {
                let result = store.get_exact(key)?;
                Ok(result)
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::get_exact(store, key).await?;
                Ok(result)
            }
        }
    }

    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_many_exact(store, keys).await,
            PsyStore::Lmdbx(store) => {
                let result = store.get_many_exact(keys)?;
                Ok(result)
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::get_many_exact(store, keys).await?;
                Ok(result)
            }
        }
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_leq(store, key, fuzzy_bytes).await,
            PsyStore::Lmdbx(store) => {
                let result = store.get_leq(key, fuzzy_bytes)?;
                Ok(result)
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::get_leq(store, key, fuzzy_bytes).await?;
                Ok(result)
            }
        }
    }

    async fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_fuzzy_range_leq_kv(store, key, fuzzy_bytes).await,
            PsyStore::Lmdbx(store) => {
                let result = store.get_fuzzy_range_leq_kv(key, fuzzy_bytes)?;
                Ok(result)
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::get_fuzzy_range_leq_kv(store, key, fuzzy_bytes).await?;
                Ok(result)
            }
        }
    }

    async fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_leq_kv(store, key, fuzzy_bytes).await,
            PsyStore::Lmdbx(store) => {
                let result = store.get_leq_kv(key, fuzzy_bytes)?;
                Ok(result)
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::get_leq_kv(store, key, fuzzy_bytes).await?;
                Ok(result)
            }
        }
    }

    async fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_many_leq(store, keys, fuzzy_bytes).await,
            PsyStore::Lmdbx(store) => {
                let result = store.get_many_leq(keys, fuzzy_bytes)?;
                Ok(result)
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::get_many_leq(store, keys, fuzzy_bytes).await?;
                Ok(result)
            }
        }
    }

    async fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>>> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::get_many_leq_kv(store, keys, fuzzy_bytes).await,
            PsyStore::Lmdbx(store) => {
                let result = store.get_many_leq_kv(keys, fuzzy_bytes)?;
                Ok(result)
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::get_many_leq_kv(store, keys, fuzzy_bytes).await?;
                Ok(result)
            }
        }
    }

    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::set(store, key, value).await,
            PsyStore::Lmdbx(store) => {
                store.set(key, value)?;
                Ok(())
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::set(store, key, value).await?;
                Ok(result)
            }
        }
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::set_ref(store, key, value).await,
            PsyStore::Lmdbx(store) => {
                store.set_ref(key, value)?;
                Ok(())
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::set_ref(store, key, value).await?;
                Ok(result)
            }
        }
    }

    async fn set_many_ref<'a>(&self, items: &[kvq::traits::KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::set_many_ref(store, items).await,
            PsyStore::Lmdbx(store) => {
                store.set_many_ref(items)?;
                Ok(())
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::set_many_ref(store, items).await?;
                Ok(result)
            }
        }
    }

    async fn set_many_vec(&self, items: Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::set_many_vec(store, items).await,
            PsyStore::Lmdbx(store) => {
                store.set_many_vec(items)?;
                Ok(())
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::set_many_vec(store, items).await?;
                Ok(result)
            }
        }
    }

    async fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::delete(store, key).await,
            PsyStore::Lmdbx(store) => {
                let result = store.delete(key)?;
                Ok(result)
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::delete(store, key).await?;
                Ok(result)
            }
        }
    }

    async fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::delete_many(store, keys).await,
            PsyStore::Lmdbx(store) => {
                let result = store.delete_many(keys)?;
                Ok(result)
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::delete_many(store, keys).await?;
                Ok(result)
            }
        }
    }

    async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::set_many_split_ref(store, keys, values).await,
            PsyStore::Lmdbx(store) => {
                store.set_many_split_ref(keys, values)?;
                Ok(())
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::set_many_split_ref(store, keys, values).await?;
                Ok(result)
            }
        }
    }

    async fn set_and_delete_many(&self, keys_to_set: &[KVQPair<&Vec<u8>, &Vec<u8>>], keys_to_delete: &[Vec<u8>]) -> anyhow::Result<()> {
        match self {
            PsyStore::Scylla(store) => <ScyllaStore as KVQBinaryStoreAsync>::set_and_delete_many(store, keys_to_set, keys_to_delete).await,
            PsyStore::Lmdbx(store) => {
                store.set_and_delete_many(keys_to_set, keys_to_delete)?;
                Ok(())
            }
            PsyStore::TiKV(store) => {
                let result = <TiKVStore as KVQBinaryStoreAsync>::set_and_delete_many(store, keys_to_set, keys_to_delete).await?;
                Ok(result)
            }
        }
    }
}
