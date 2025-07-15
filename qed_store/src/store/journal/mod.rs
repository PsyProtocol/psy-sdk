use kvq::traits::KVQPair;
use std::sync::Arc;
use ambassador::Delegate;
use kvq::cache::{KVQBinaryStoreCached, KVQBinaryStoreCachedTrait};
use kvq::traits::KVQBinaryStore;
use kvq::traits::ambassador_impl_KVQBinaryStore;

pub trait Journal: KVQBinaryStore {
    fn commit(&self) -> anyhow::Result<()>;
    fn rollback(&self) -> anyhow::Result<()>;
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
    fn commit(&self) -> anyhow::Result<()> {
        self.inner.flush_simple()
    }

    fn rollback(&self) -> anyhow::Result<()> {
        let mut map = self.inner.map.write().unwrap();
        map.clear();
        Ok(())
    }
}