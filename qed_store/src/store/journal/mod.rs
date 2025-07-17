use kvq::traits::{KVQBinaryStoreAsync, KVQPair};
use std::sync::Arc;
use ambassador::Delegate;
use async_trait::async_trait;
use auto_impl::auto_impl;
use kvq::cache::{KVQBinaryStoreCached, KVQBinaryStoreCachedAsync, KVQBinaryStoreCachedTrait, KVQBinaryStoreCachedTraitAsync};
use kvq::traits::KVQBinaryStore;
use kvq::traits::ambassador_impl_KVQBinaryStore;

#[auto_impl(&, Box, Arc)]
pub trait Journal: KVQBinaryStore {
    fn commit(&self, _checkpoint_id: u64) -> anyhow::Result<()>;
    fn rollback(&self, _checkpoint_id: u64) -> anyhow::Result<()>;
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
            inner: KVQBinaryStoreCached::new(Arc::new(store))
        }
    }
}

impl<S: KVQBinaryStore> Journal for JournalStore<S> {
    fn commit(&self, _checkpoint_id: u64) -> anyhow::Result<()> {
        self.inner.flush_simple()
    }

    fn rollback(&self, _checkpoint_id: u64) -> anyhow::Result<()> {
        self.inner.clear_cache();
        Ok(())
    }
    
    fn get_base_store(&self) -> &dyn KVQBinaryStore {
        self.inner.store.as_ref()
    }
}

#[async_trait]
#[auto_impl(&, Box, Arc)]
pub trait JournalAsync: KVQBinaryStoreAsync {
    async fn commit(&self, _checkpoint_id: u64) -> anyhow::Result<()>;
    async fn rollback(&self, _checkpoint_id: u64) -> anyhow::Result<()>;
    async fn get_base_store(&self) -> &dyn KVQBinaryStoreAsync;
}


pub struct JournalStoreAsync<S: KVQBinaryStoreAsync+ Send + Sync> {
    inner: KVQBinaryStoreCachedAsync<S>,
}

impl<S: KVQBinaryStoreAsync+ Send + Sync> JournalStoreAsync<S> {
    pub fn new(store: S) -> Self {
        Self {
            inner: KVQBinaryStoreCachedAsync::new(Arc::new(store))
        }
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
impl<S: KVQBinaryStoreAsync+ Send + Sync> JournalAsync for JournalStoreAsync<S> {
    async fn commit(&self, _checkpoint_id: u64) -> anyhow::Result<()> {
        self.inner.flush_simple().await
    }

    async fn rollback(&self, _checkpoint_id: u64) -> anyhow::Result<()> {
        self.inner.clear_cache().await;
        Ok(())
    }
    
    async fn get_base_store(&self) -> &dyn KVQBinaryStoreAsync {
        self.inner.store.as_ref()
    }
}