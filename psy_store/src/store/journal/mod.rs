pub mod backup_journal;

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use auto_impl::auto_impl;
pub use backup_journal::{BackupHandler, BackupJournalStore, BackupRequest};
use kvq::{
    cache::{CacheValueType, KVQSimpleCachedStore},
    traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair},
};

#[async_trait]
#[auto_impl(&, Box, Arc)]
pub trait Journal: KVQBinaryStoreAsync + BackupHandler {
    async fn commit(&self, _checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)>;
    async fn is_committed(&self) -> bool;
    async fn rollback(&self, _checkpoint_id: u64) -> anyhow::Result<()>;
    async fn restore_cache(&self, cache: Vec<u8>) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_cache(&self) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(Option::None)
    }
    async fn get_base_store(&self) -> &dyn KVQBinaryStoreAsync;
}

#[derive(Clone)]
pub struct JournalStore<S: KVQBinaryStoreAsync + Send + Sync> {
    inner: KVQSimpleCachedStore<S>,
}

#[async_trait]
impl<S: KVQBinaryStoreAsync + Send + Sync> BackupHandler for JournalStore<S> {
    async fn handle_backup(&self, request: BackupRequest) -> anyhow::Result<()> {
        tracing::info!("JournalStore does not need handle backup request: {:?}", request);
        Ok(())
    }
}

impl<S: KVQBinaryStoreAsync + Send + Sync> JournalStore<S> {
    pub fn new(store: S) -> Self {
        Self {
            inner: KVQSimpleCachedStore::new(Arc::new(store)),
        }
    }

    pub async fn get_cache_snapshot(&self) -> BTreeMap<Vec<u8>, CacheValueType> {
        self.inner.map.read().await.clone()
    }

    pub async fn restore_from_snapshot(&self, cache: BTreeMap<Vec<u8>, CacheValueType>) {
        let mut map = self.inner.map.write().await;
        *map = cache;
    }
}

#[async_trait]
impl<S: KVQBinaryStoreAsync + Send + Sync> KVQBinaryStoreAsync for JournalStore<S> {
    async fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::get_exact(&self.inner, key).await
    }

    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::get_exact_if_exists(&self.inner, key).await
    }

    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::get_many_exact(&self.inner, keys).await
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::get_leq(&self.inner, key, fuzzy_bytes).await
    }

    async fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::get_leq_kv(&self.inner, key, fuzzy_bytes).await
    }

    async fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::get_many_leq(&self.inner, keys, fuzzy_bytes).await
    }

    async fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::get_many_leq_kv(&self.inner, keys, fuzzy_bytes).await
    }

    async fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::get_fuzzy_range_leq_kv(&self.inner, key, fuzzy_bytes).await
    }

    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::set(&self.inner, key, value).await
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::set_ref(&self.inner, key, value).await
    }

    async fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::set_many_ref(&self.inner, items).await
    }

    async fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::set_many_vec(&self.inner, items).await
    }

    async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::set_many_split_ref(&self.inner, keys, values).await
    }

    async fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::delete(&self.inner, key).await
    }

    async fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::delete_many(&self.inner, keys).await
    }

    async fn set_and_delete_many(&self, sets: &[KVQPair<&Vec<u8>, &Vec<u8>>], deletes: &[Vec<u8>]) -> anyhow::Result<()> {
        <KVQSimpleCachedStore<S> as KVQBinaryStoreAsync>::set_and_delete_many(&self.inner, sets, deletes).await
    }
}

#[async_trait]
impl<S: KVQBinaryStoreAsync + Send + Sync> Journal for JournalStore<S> {
    async fn commit(&self, _checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        self.inner.flush_simple(_checkpoint_id).await
    }

    async fn is_committed(&self) -> bool {
        self.inner.map.read().await.is_empty()
    }

    async fn rollback(&self, _checkpoint_id: u64) -> anyhow::Result<()> {
        self.inner.clear_cache().await;
        Ok(())
    }

    async fn restore_cache(&self, cache: Vec<u8>) -> anyhow::Result<()> {
        let cache_value = bincode::deserialize(&cache)?;
        self.restore_from_snapshot(cache_value).await;
        Ok(())
    }

    async fn get_cache(&self) -> anyhow::Result<Option<Vec<u8>>> {
        let cache = self.get_cache_snapshot().await;
        if cache.is_empty() {
            return Ok(Option::None);
        }
        Ok(Some(bincode::serialize(&cache)?))
    }

    async fn get_base_store(&self) -> &dyn KVQBinaryStoreAsync {
        self.inner.store.as_ref()
    }
}

impl<S: KVQBinaryStoreAsync + Send + Sync> KVQBinaryStore for JournalStore<S> {
    fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_exact(self, key).await })
        })
    }

    fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_exact_if_exists(self, key).await })
        })
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_many_exact(self, keys).await })
        })
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_leq(self, key, fuzzy_bytes).await })
        })
    }

    fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_leq_kv(self, key, fuzzy_bytes).await })
        })
    }

    fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_many_leq(self, keys, fuzzy_bytes).await })
        })
    }

    fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_many_leq_kv(self, keys, fuzzy_bytes).await })
        })
    }

    fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::get_fuzzy_range_leq_kv(self, key, fuzzy_bytes).await })
        })
    }

    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::set(self, key, value).await })
        })
    }

    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::set_ref(self, key, value).await })
        })
    }

    fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::set_many_ref(self, items).await })
        })
    }

    fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::set_many_vec(self, items).await })
        })
    }

    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::set_many_split_ref(self, keys, values).await })
        })
    }

    fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::delete(self, key).await })
        })
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::delete_many(self, keys).await })
        })
    }
}
