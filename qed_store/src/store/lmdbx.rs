use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct KVQlibmdbxStore {
    inner: Arc<KVQSimpleMemoryBackingStore>,
}

impl KVQlibmdbxStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(KVQSimpleMemoryBackingStore::new()),
        }
    }

    pub fn new_write_with_size(_path: &str, _size_gb: usize) -> anyhow::Result<Self> {
        Ok(Self::new())
    }

    pub fn new_write(_path: &str) -> anyhow::Result<Self> {
        Ok(Self::new())
    }

    pub fn new_read(_path: &str) -> anyhow::Result<Self> {
        Ok(Self::new())
    }
}

impl std::ops::Deref for KVQlibmdbxStore {
    type Target = KVQSimpleMemoryBackingStore;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl kvq::traits::KVQBinaryStore for KVQlibmdbxStore {
    fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.get_exact_if_exists(key)
    }

    fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        self.inner.get_exact(key)
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        self.inner.get_many_exact(keys)
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        self.inner.get_leq(key, fuzzy_bytes)
    }

    fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>> {
        self.inner.get_fuzzy_range_leq_kv(key, fuzzy_bytes)
    }

    fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>> {
        self.inner.get_leq_kv(key, fuzzy_bytes)
    }

    fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        self.inner.get_many_leq(keys, fuzzy_bytes)
    }

    fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>>> {
        self.inner.get_many_leq_kv(keys, fuzzy_bytes)
    }

    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        self.inner.set(key, value)
    }

    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        self.inner.set_ref(key, value)
    }

    fn set_many_ref<'a>(&self, items: &[kvq::traits::KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        self.inner.set_many_ref(items)
    }

    fn set_many_vec(&self, items: Vec<kvq::traits::KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        self.inner.set_many_vec(items)
    }

    fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        self.inner.delete(key)
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        self.inner.delete_many(keys)
    }

    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        self.inner.set_many_split_ref(keys, values)
    }
}