#[cfg(test)]
pub mod test_helpers {
    use async_trait::async_trait;

    use crate::{
        memory::simple::KVQSimpleMemoryBackingStore,
        traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair},
    };

    /// A wrapper that makes KVQSimpleMemoryBackingStore async for testing
    pub struct AsyncMemoryStore {
        inner: KVQSimpleMemoryBackingStore,
    }

    impl AsyncMemoryStore {
        pub fn new() -> Self {
            Self {
                inner: KVQSimpleMemoryBackingStore::new(),
            }
        }

        // Helper method to populate with initial data
        pub fn with_data(data: Vec<(Vec<u8>, Vec<u8>)>) -> Self {
            let store = Self::new();
            for (k, v) in data {
                store.inner.set_ref(&k, &v).unwrap();
            }
            store
        }
    }

    #[async_trait]
    impl KVQBinaryStoreAsync for AsyncMemoryStore {
        async fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
            Ok(self.inner.get_exact_if_exists(key)?)
        }

        async fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
            Ok(self.inner.get_exact(key)?)
        }

        async fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
            Ok(self.inner.get_many_exact(keys)?)
        }

        async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
            Ok(self.inner.get_leq(key, fuzzy_bytes)?)
        }

        async fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
            Ok(self.inner.get_fuzzy_range_leq_kv(key, fuzzy_bytes)?)
        }

        async fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
            Ok(self.inner.get_leq_kv(key, fuzzy_bytes)?)
        }

        async fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
            Ok(self.inner.get_many_leq(keys, fuzzy_bytes)?)
        }

        async fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
            Ok(self.inner.get_many_leq_kv(keys, fuzzy_bytes)?)
        }

        async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
            Ok(self.inner.set(key, value)?)
        }

        async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
            Ok(self.inner.set_ref(key, value)?)
        }

        async fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
            Ok(self.inner.set_many_ref(items)?)
        }

        async fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
            Ok(self.inner.set_many_vec(items)?)
        }

        async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
            Ok(self.inner.set_many_split_ref(keys, values)?)
        }

        async fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
            Ok(self.inner.delete(key)?)
        }

        async fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
            Ok(self.inner.delete_many(keys)?)
        }
    }
}
