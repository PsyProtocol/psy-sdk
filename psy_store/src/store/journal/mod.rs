use std::{collections::BTreeMap, sync::Arc};

use ambassador::Delegate;
use async_trait::async_trait;
use auto_impl::auto_impl;
use kvq::{
    cache::{CacheValueType, KVQBinaryStoreCached, KVQBinaryStoreCachedAsync, KVQBinaryStoreCachedTrait, KVQBinaryStoreCachedTraitAsync},
    traits::{ambassador_impl_KVQBinaryStore, KVQBinaryStore, KVQBinaryStoreAsync, KVQPair},
};

#[auto_impl(&, Box, Arc)]
pub trait Journal: KVQBinaryStore {
    fn commit(&self, _checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)>;
    fn is_committed(&self) -> bool;
    fn rollback(&self, _checkpoint_id: u64) -> anyhow::Result<()>;
    fn save_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        Ok(())
    }
    fn restore_snapshot(&self, snapshot: Vec<u8>) -> anyhow::Result<()> {
        Ok(())
    }
    fn cleanup_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        Ok(())
    }
    fn get_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(Option::None)
    }
    fn get_base_store(&self) -> &dyn KVQBinaryStore;
}

#[derive(Clone, Delegate)]
#[delegate(KVQBinaryStore)]
pub struct JournalStore<S: KVQBinaryStore> {
    #[delegate(KVQBinaryStore)]
    inner: KVQBinaryStoreCached<S>,
}

impl<S: KVQBinaryStore> JournalStore<S> {
    pub fn new(store: S) -> Self {
        Self {
            inner: KVQBinaryStoreCached::new(Arc::new(store)),
        }
    }
    fn snapshot_key(&self, checkpoint_id: u64) -> Vec<u8> {
        format!("CACHE_SNAPSHOT:{}", checkpoint_id).into_bytes()
    }

    pub fn get_cache_snapshot(&self) -> BTreeMap<Vec<u8>, CacheValueType> {
        self.inner.map.read().clone()
    }

    pub fn restore_from_snapshot(&self, snapshot: BTreeMap<Vec<u8>, CacheValueType>) {
        let mut map = self.inner.map.write();
        *map = snapshot;
    }
}

impl<S: KVQBinaryStore> Journal for JournalStore<S> {
    fn commit(&self, _checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        self.inner.flush_simple(_checkpoint_id)
    }

    fn is_committed(&self) -> bool {
        self.inner.map.read().is_empty()
    }

    fn rollback(&self, _checkpoint_id: u64) -> anyhow::Result<()> {
        self.inner.clear_cache();
        Ok(())
    }
    fn save_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        let snapshot = self.get_cache_snapshot();
        let snapshot_value = bincode::serialize(&snapshot)?;
        self.get_base_store().set(self.snapshot_key(checkpoint_id), snapshot_value)
    }
    fn restore_snapshot(&self, snapshot: Vec<u8>) -> anyhow::Result<()> {
        let snapshot_value = bincode::deserialize(&snapshot)?;
        self.restore_from_snapshot(snapshot_value);
        Ok(())
    }
    fn cleanup_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        self.get_base_store().delete(&self.snapshot_key(checkpoint_id))?;
        Ok(())
    }
    fn get_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<Option<Vec<u8>>> {
        self.get_base_store().get_exact_if_exists(&self.snapshot_key(checkpoint_id))
    }
    fn get_base_store(&self) -> &dyn KVQBinaryStore {
        self.inner.store.as_ref()
    }
}

#[async_trait]
#[auto_impl(&, Box, Arc)]
pub trait JournalAsync: KVQBinaryStoreAsync {
    async fn commit(&self, _checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)>;
    async fn is_committed(&self) -> bool;
    async fn rollback(&self, _checkpoint_id: u64) -> anyhow::Result<()>;
    async fn save_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        Ok(())
    }
    async fn restore_snapshot(&self, snapshot: Vec<u8>) -> anyhow::Result<()> {
        Ok(())
    }
    async fn cleanup_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(Option::None)
    }
    async fn get_base_store(&self) -> &dyn KVQBinaryStoreAsync;
}

pub struct JournalStoreAsync<S: KVQBinaryStoreAsync + Send + Sync> {
    inner: KVQBinaryStoreCachedAsync<S>,
}

impl<S: KVQBinaryStoreAsync + Send + Sync> JournalStoreAsync<S> {
    pub fn new(store: S) -> Self {
        Self {
            inner: KVQBinaryStoreCachedAsync::new(Arc::new(store)),
        }
    }

    fn snapshot_key(&self, checkpoint_id: u64) -> Vec<u8> {
        format!("CACHE_SNAPSHOT:{}", checkpoint_id).into_bytes()
    }

    pub async fn get_cache_snapshot(&self) -> BTreeMap<Vec<u8>, CacheValueType> {
        self.inner.map.read().await.clone()
    }

    pub async fn restore_from_snapshot(&self, snapshot: BTreeMap<Vec<u8>, CacheValueType>) {
        let mut map = self.inner.map.write().await;
        *map = snapshot;
    }
}

#[async_trait]
impl<S: KVQBinaryStoreAsync + Send + Sync> KVQBinaryStoreAsync for JournalStoreAsync<S> {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.get_exact_if_exists(key).await
    }

    async fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        self.inner.get_exact(key).await
    }

    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        self.inner.get_many_exact(keys).await
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.get_leq(key, fuzzy_bytes).await
    }

    async fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        self.inner.get_fuzzy_range_leq_kv(key, fuzzy_bytes).await
    }

    async fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        self.inner.get_leq_kv(key, fuzzy_bytes).await
    }

    async fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        self.inner.get_many_leq(keys, fuzzy_bytes).await
    }

    async fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        self.inner.get_many_leq_kv(keys, fuzzy_bytes).await
    }

    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        self.inner.set(key, value).await
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        self.inner.set_ref(key, value).await
    }

    async fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        self.inner.set_many_ref(items).await
    }

    async fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        self.inner.set_many_vec(items).await
    }

    async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        self.inner.set_many_split_ref(keys, values).await
    }

    async fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        self.inner.delete(key).await
    }

    async fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        self.inner.delete_many(keys).await
    }
}

#[async_trait]
impl<S: KVQBinaryStoreAsync + Send + Sync> JournalAsync for JournalStoreAsync<S> {
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

    async fn save_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        let snapshot = self.get_cache_snapshot().await;
        let snapshot_value = bincode::serialize(&snapshot)?;
        self.get_base_store().await.set(self.snapshot_key(checkpoint_id), snapshot_value).await
    }
    async fn restore_snapshot(&self, snapshot: Vec<u8>) -> anyhow::Result<()> {
        let snapshot_value = bincode::deserialize(&snapshot)?;
        self.restore_from_snapshot(snapshot_value).await;
        Ok(())
    }
    async fn cleanup_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<()> {
        self.get_base_store().await.delete(&self.snapshot_key(checkpoint_id)).await?;
        Ok(())
    }
    async fn get_snapshot(&self, checkpoint_id: u64) -> anyhow::Result<Option<Vec<u8>>> {
        self.get_base_store().await.get_exact_if_exists(&self.snapshot_key(checkpoint_id)).await
    }

    async fn get_base_store(&self) -> &dyn KVQBinaryStoreAsync {
        self.inner.store.as_ref()
    }
}
