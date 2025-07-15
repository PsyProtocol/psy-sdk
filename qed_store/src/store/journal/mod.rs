use kvq::traits::KVQPair;
use std::sync::Arc;
use ambassador::Delegate;
use auto_impl::auto_impl;
use kvq::cache::{KVQBinaryStoreCached, KVQBinaryStoreCachedTrait};
use kvq::traits::KVQBinaryStore;
use kvq::traits::ambassador_impl_KVQBinaryStore;

#[auto_impl(&, Box, Arc)]
pub trait Journal: KVQBinaryStore {
    fn commit(&self, _checkpoint_id: u64) -> anyhow::Result<()>;
    fn rollback(&self, _checkpoint_id: u64) -> anyhow::Result<()>;
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
}