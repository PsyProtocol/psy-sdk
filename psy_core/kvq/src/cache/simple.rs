use std::{
    collections::{BTreeMap, Bound::Included},
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::{
    cache::{CacheValueType, KVQBinaryStoreCache, KVQBinaryStoreCacheAsync},
    traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair},
};

#[derive(Clone)]
pub struct KVQSimpleCachedStore<S: KVQBinaryStoreAsync + Send + Sync> {
    pub store: Arc<S>,
    pub map: Arc<RwLock<BTreeMap<Vec<u8>, CacheValueType>>>,
    pub proper_delete_return: bool,
}

impl<S: KVQBinaryStoreAsync + Send + Sync> KVQSimpleCachedStore<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self {
            store,
            map: Arc::new(RwLock::new(BTreeMap::new())),
            proper_delete_return: false,
        }
    }

    pub fn with_proper_delete_return(mut self, enabled: bool) -> Self {
        self.proper_delete_return = enabled;
        self
    }

    async fn get_leq_from_cache(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> Option<KVQPair<Vec<u8>, Vec<u8>>> {
        let map = self.map.read().await;

        if fuzzy_bytes == 0 {
            for (k, v) in map.range(..=key.clone()).rev() {
                if let CacheValueType::Bytes(b) = v {
                    return Some(KVQPair {
                        key: k.clone(),
                        value: b.clone(),
                    });
                }
            }
        } else {
            let key_end = key.to_vec();
            let mut base_key = key.to_vec();
            let key_len = base_key.len();

            for i in 0..fuzzy_bytes {
                base_key[key_len - i - 1] = 0;
            }

            for (k, v) in map.range((Included(base_key), Included(key_end))).rev() {
                if let CacheValueType::Bytes(b) = v {
                    return Some(KVQPair {
                        key: k.clone(),
                        value: b.clone(),
                    });
                }
            }
        }

        None
    }

    pub async fn clear_cache(&self) {
        self.map.write().await.clear();
    }

    pub async fn flush_simple(&self, checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        let (keys_to_set, removed_keys) = {
            let map = self.map.write().await;
            let keys_to_set: Vec<KVQPair<Vec<u8>, Vec<u8>>> = map
                .iter()
                .filter(|(_, vt)| matches!(vt, CacheValueType::Bytes(_)))
                .map(|(k, vt)| {
                    let mut k = k.clone();
                    if let Some(checkpoint_id) = checkpoint_id {
                        let len = k.len();
                        k[len - 8..].copy_from_slice(&checkpoint_id.to_be_bytes())
                    }
                    match vt {
                        CacheValueType::Bytes(b) => Ok(KVQPair { key: k, value: b.clone() }),
                        CacheValueType::Removed => Err(anyhow::anyhow!("Cannot flush changes with removed keys")),
                    }
                })
                .collect::<anyhow::Result<Vec<_>>>()?;

            let removed_keys = map
                .iter()
                .filter(|(_, vt)| matches!(vt, CacheValueType::Removed))
                .map(|(k, _)| {
                    let mut k = k.to_owned();
                    if let Some(checkpoint_id) = checkpoint_id {
                        let len = k.len();
                        k[len - 8..].copy_from_slice(&checkpoint_id.to_be_bytes())
                    }
                    k
                })
                .collect::<Vec<_>>();

            (keys_to_set, removed_keys)
        };

        let keys_to_set_ref: Vec<KVQPair<&Vec<u8>, &Vec<u8>>> = keys_to_set
            .iter()
            .map(|kv| KVQPair {
                key: &kv.key,
                value: &kv.value,
            })
            .collect();

        self.store.set_and_delete_many(&keys_to_set_ref, &removed_keys).await?;

        self.map.write().await.clear();
        Ok((keys_to_set, removed_keys))
    }

    async fn flush_changes(&self) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        let mut map = self.map.write().await;
        let keys_to_set: Vec<KVQPair<&Vec<u8>, &Vec<u8>>> = map
            .iter()
            .filter(|(_, vt)| matches!(vt, CacheValueType::Bytes(_)))
            .map(|(k, vt)| match vt {
                CacheValueType::Bytes(b) => Ok(KVQPair { key: k, value: b }),
                CacheValueType::Removed => Err(anyhow::anyhow!("Cannot flush changes with removed keys")),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let removed_keys = map
            .iter()
            .filter(|(_, vt)| matches!(vt, CacheValueType::Removed))
            .map(|(k, _)| k.to_owned())
            .collect::<Vec<_>>();

        let set_keys = keys_to_set
            .iter()
            .map(|x| KVQPair {
                key: x.key.to_owned(),
                value: x.value.to_owned(),
            })
            .collect::<Vec<_>>();

        map.clear();
        Ok((set_keys, removed_keys))
    }

    async fn is_removed(&self, key: &Vec<u8>) -> bool {
        matches!(self.map.read().await.get(key), Some(CacheValueType::Removed))
    }

    async fn get_non_removed_keys(&self) -> Vec<Vec<u8>> {
        self.map
            .read()
            .await
            .iter()
            .filter(|(_, vt)| matches!(vt, CacheValueType::Bytes(_)))
            .map(|(k, _)| k.to_owned())
            .collect::<Vec<_>>()
    }

    async fn get_removed_keys(&self) -> Vec<Vec<u8>> {
        self.map
            .read()
            .await
            .iter()
            .filter(|(_, vt)| matches!(vt, CacheValueType::Removed))
            .map(|(k, _)| k.to_owned())
            .collect::<Vec<_>>()
    }
}

impl<S: KVQBinaryStoreAsync + Send + Sync> KVQBinaryStoreCache for KVQSimpleCachedStore<S> {
    fn flush_changes(&self) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(async { self.flush_changes().await }))
    }

    fn clear_cache(&self) {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(async { self.clear_cache().await }))
    }

    fn flush_simple(&self, checkpoint_id: Option<u64>) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(async { self.flush_simple(checkpoint_id).await }))
    }

    fn is_removed(&self, key: &Vec<u8>) -> bool {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(async { self.is_removed(key).await }))
    }

    fn get_non_removed_keys(&self) -> Vec<Vec<u8>> {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(async { self.get_non_removed_keys().await }))
    }

    fn get_removed_keys(&self) -> Vec<Vec<u8>> {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(async { self.get_removed_keys().await }))
    }
}

#[async_trait::async_trait]
impl<S: KVQBinaryStoreAsync + Send + Sync> KVQBinaryStoreAsync for KVQSimpleCachedStore<S> {
    async fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        match self.map.read().await.get(key) {
            Some(CacheValueType::Bytes(b)) => Ok(b.to_owned()),
            Some(CacheValueType::Removed) => anyhow::bail!("Key {} not found", hex::encode(key)),
            None => self.store.get_exact(key).await,
        }
    }

    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        match self.map.read().await.get(key) {
            Some(CacheValueType::Bytes(b)) => Ok(Some(b.to_owned())),
            Some(CacheValueType::Removed) => Ok(None),
            None => self.store.get_exact_if_exists(key).await,
        }
    }

    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            result.push(self.store.get_exact(key).await?);
        }
        Ok(result)
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        if fuzzy_bytes > key.len() {
            anyhow::bail!("Fuzzy bytes must be less than or equal to key length");
        }

        let cache_candidate = self.get_leq_from_cache(key, fuzzy_bytes).await;
        let store_candidate = self.store.get_leq_kv(key, fuzzy_bytes).await?;

        Ok(match (cache_candidate, store_candidate) {
            (None, None) => None,
            (Some(c), None) => Some(c.value),
            (None, Some(s)) => {
                if self.map.read().await.get(&s.key).map_or(false, |v| matches!(v, CacheValueType::Removed)) {
                    None
                } else {
                    Some(s.value)
                }
            }
            (Some(c), Some(s)) => {
                if self.map.read().await.get(&s.key).map_or(false, |v| matches!(v, CacheValueType::Removed)) {
                    Some(c.value)
                } else if c.key >= s.key {
                    Some(c.value)
                } else {
                    Some(s.value)
                }
            }
        })
    }

    async fn get_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        if fuzzy_bytes > key.len() {
            anyhow::bail!("Fuzzy bytes must be less than or equal to key length");
        }

        let cache_candidate = self.get_leq_from_cache(key, fuzzy_bytes).await;
        let store_candidate = self.store.get_leq_kv(key, fuzzy_bytes).await?;

        Ok(match (cache_candidate, store_candidate) {
            (None, None) => None,
            (Some(c), None) => Some(c),
            (None, Some(s)) => {
                if self.map.read().await.get(&s.key).map_or(false, |v| matches!(v, CacheValueType::Removed)) {
                    None
                } else {
                    Some(s)
                }
            }
            (Some(c), Some(s)) => {
                if self.map.read().await.get(&s.key).map_or(false, |v| matches!(v, CacheValueType::Removed)) {
                    Some(c)
                } else if c.key >= s.key {
                    Some(c)
                } else {
                    Some(s)
                }
            }
        })
    }

    async fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(<Self as KVQBinaryStoreAsync>::get_leq(self, key, fuzzy_bytes).await?);
        }
        Ok(results)
    }

    async fn get_many_leq_kv(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(<Self as KVQBinaryStoreAsync>::get_leq_kv(self, key, fuzzy_bytes).await?);
        }
        Ok(results)
    }

    async fn get_fuzzy_range_leq_kv(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        if fuzzy_bytes > key.len() {
            anyhow::bail!("Fuzzy bytes must be less than or equal to key length");
        }

        let key_end = key.to_vec();
        let mut base_key = key.to_vec();
        let key_len = base_key.len();
        for i in 0..fuzzy_bytes {
            base_key[key_len - i - 1] = 0;
        }

        let map = self.map.read().await;
        let mut results = BTreeMap::new();

        for item in self.store.get_fuzzy_range_leq_kv(key, fuzzy_bytes).await? {
            results.insert(item.key.clone(), item);
        }

        for (k, v) in map.range((Included(base_key), Included(key_end))) {
            match v {
                CacheValueType::Bytes(b) => {
                    results.insert(
                        k.clone(),
                        KVQPair {
                            key: k.clone(),
                            value: b.clone(),
                        },
                    );
                }
                CacheValueType::Removed => {
                    results.remove(k);
                }
            }
        }

        Ok(results.into_values().collect())
    }

    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        self.map.write().await.insert(key, CacheValueType::Bytes(value));
        Ok(())
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        self.map.write().await.insert(key.clone(), CacheValueType::Bytes(value.clone()));
        Ok(())
    }

    async fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> anyhow::Result<()> {
        let mut map = self.map.write().await;
        for item in items {
            map.insert(item.key.clone(), CacheValueType::Bytes(item.value.clone()));
        }
        Ok(())
    }

    async fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        let mut map = self.map.write().await;
        for item in items {
            map.insert(item.key, CacheValueType::Bytes(item.value));
        }
        Ok(())
    }

    async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        if keys.len() != values.len() {
            anyhow::bail!("Keys and values must have the same length");
        }
        let mut map = self.map.write().await;
        for i in 0..keys.len() {
            map.insert(keys[i].clone(), CacheValueType::Bytes(values[i].clone()));
        }
        Ok(())
    }

    async fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        let previous = self.map.write().await.insert(key.clone(), CacheValueType::Removed);
        if previous.is_none() {
            if self.proper_delete_return {
                Ok(<Self as KVQBinaryStoreAsync>::get_exact_if_exists(self, key).await?.is_some())
            } else {
                Ok(false)
            }
        } else {
            Ok(true)
        }
    }

    async fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(<Self as KVQBinaryStoreAsync>::delete(self, key).await?);
        }
        Ok(results)
    }
}

impl<S: KVQBinaryStoreAsync + Send + Sync> KVQBinaryStore for KVQSimpleCachedStore<S> {
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

    fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::delete(self, key).await }))
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::delete_many(self, keys).await })
        })
    }

    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { <Self as KVQBinaryStoreAsync>::set_many_split_ref(self, keys, values).await })
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::memory::simple::KVQSimpleMemoryBackingStore;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_basic_operations() {
        let backing = KVQSimpleCachedStore::new(Arc::new(KVQSimpleMemoryBackingStore::new()));
        let store = KVQSimpleCachedStore::new(Arc::new(backing));

        <KVQSimpleCachedStore<_> as KVQBinaryStore>::set_ref(&store, &vec![1], &vec![10]).unwrap();
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_exact(&store, &vec![1]).unwrap(),
            vec![10]
        );

        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_exact_if_exists(&store, &vec![1]).unwrap(),
            Some(vec![10])
        );
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_exact_if_exists(&store, &vec![2]).unwrap(),
            None
        );

        assert!(<KVQSimpleCachedStore<_> as KVQBinaryStore>::delete(&store, &vec![1]).unwrap());
        assert!(<KVQSimpleCachedStore<_> as KVQBinaryStore>::get_exact(&store, &vec![1]).is_err());
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_exact_if_exists(&store, &vec![1]).unwrap(),
            None
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cache_functionality() {
        let backing = KVQSimpleCachedStore::new(Arc::new(KVQSimpleMemoryBackingStore::new()));
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![1], &vec![10]).unwrap();
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![3], &vec![30]).unwrap();

        let store = KVQSimpleCachedStore::new(Arc::new(backing));

        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![3], 0).unwrap(),
            Some(vec![30])
        );
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![4], 0).unwrap(),
            Some(vec![30])
        );
        assert_eq!(<KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![0], 0).unwrap(), None);

        <KVQSimpleCachedStore<_> as KVQBinaryStore>::set_ref(&store, &vec![2], &vec![20]).unwrap();
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![2], 0).unwrap(),
            Some(vec![20])
        );

        <KVQSimpleCachedStore<_> as KVQBinaryStore>::delete(&store, &vec![1]).unwrap();
        assert_eq!(<KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![1], 0).unwrap(), None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_flush_operations() {
        let backing = KVQSimpleCachedStore::new(Arc::new(KVQSimpleMemoryBackingStore::new()));
        let store = KVQSimpleCachedStore::new(Arc::new(backing.clone()));

        <KVQSimpleCachedStore<_> as KVQBinaryStore>::set_ref(&store, &vec![1], &vec![10]).unwrap();
        <KVQSimpleCachedStore<_> as KVQBinaryStore>::set_ref(&store, &vec![2], &vec![20]).unwrap();
        <KVQSimpleCachedStore<_> as KVQBinaryStore>::delete(&store, &vec![3]).unwrap();

        let (sets, deletes) = <KVQSimpleCachedStore<_> as KVQBinaryStoreCache>::flush_changes(&store).unwrap();
        assert_eq!(sets.len(), 2);
        assert_eq!(deletes.len(), 1);

        <KVQSimpleCachedStore<_> as KVQBinaryStoreCache>::clear_cache(&store);
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_exact_if_exists(&store, &vec![1]).unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn test_get_leq_basic() {
        let backing = KVQSimpleCachedStore::new(Arc::new(KVQSimpleMemoryBackingStore::new()));
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![1], &vec![10]).unwrap();
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![3], &vec![30]).unwrap();
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![5], &vec![50]).unwrap();

        let store = KVQSimpleCachedStore::new(Arc::new(backing));

        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![3], 0).unwrap(),
            Some(vec![30])
        );
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![4], 0).unwrap(),
            Some(vec![30])
        );
        assert_eq!(<KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![0], 0).unwrap(), None);
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![6], 0).unwrap(),
            Some(vec![50])
        );
    }

    #[tokio::test]
    async fn test_get_leq_with_cache_updates() {
        let backing = KVQSimpleCachedStore::new(Arc::new(KVQSimpleMemoryBackingStore::new()));
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![1], &vec![10]).unwrap();
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![5], &vec![50]).unwrap();

        let store = KVQSimpleCachedStore::new(Arc::new(backing));

        <KVQSimpleCachedStore<_> as KVQBinaryStore>::set_ref(&store, &vec![3], &vec![30]).unwrap();
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![4], 0).unwrap(),
            Some(vec![30])
        );

        <KVQSimpleCachedStore<_> as KVQBinaryStore>::delete(&store, &vec![3]).unwrap();
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![4], 0).unwrap(),
            Some(vec![10])
        );
    }

    #[tokio::test]
    async fn test_get_fuzzy_range_leq_kv_with_fuzzy() {
        let backing = KVQSimpleCachedStore::new(Arc::new(KVQSimpleMemoryBackingStore::new()));
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![1, 0], &vec![10]).unwrap();
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![1, 1], &vec![11]).unwrap();
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![1, 2], &vec![12]).unwrap();
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![2, 0], &vec![20]).unwrap();

        let store = KVQSimpleCachedStore::new(Arc::new(backing.clone()));

        let result = <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_fuzzy_range_leq_kv(&store, &vec![1, 3], 1).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].key, vec![1, 0]);
        assert_eq!(result[1].key, vec![1, 1]);
        assert_eq!(result[2].key, vec![1, 2]);

        <KVQSimpleCachedStore<_> as KVQBinaryStore>::delete(&store, &vec![1, 1]).unwrap();
        <KVQSimpleCachedStore<_> as KVQBinaryStore>::set_ref(&store, &vec![1, 3], &vec![13]).unwrap();

        let result2 = <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_fuzzy_range_leq_kv(&store, &vec![1, 3], 1).unwrap();
        assert_eq!(result2.len(), 3);
        assert_eq!(result2[0].key, vec![1, 0]);
        assert_eq!(result2[1].key, vec![1, 2]);
        assert_eq!(result2[2].key, vec![1, 3]);
    }

    #[tokio::test]
    async fn test_get_leq_kv() {
        let backing = KVQSimpleCachedStore::new(Arc::new(KVQSimpleMemoryBackingStore::new()));
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![1], &vec![10]).unwrap();
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![3], &vec![30]).unwrap();

        let store = KVQSimpleCachedStore::new(Arc::new(backing));

        let result = <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq_kv(&store, &vec![2], 0).unwrap();
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.key, vec![1]);
        assert_eq!(pair.value, vec![10]);
    }

    #[tokio::test]
    async fn test_cache_overrides_backing() {
        let backing = KVQSimpleCachedStore::new(Arc::new(KVQSimpleMemoryBackingStore::new()));
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![1], &vec![10]).unwrap();

        let store = KVQSimpleCachedStore::new(Arc::new(backing));

        <KVQSimpleCachedStore<_> as KVQBinaryStore>::set_ref(&store, &vec![1], &vec![20]).unwrap();
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_exact(&store, &vec![1]).unwrap(),
            vec![20]
        );
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![1], 0).unwrap(),
            Some(vec![20])
        );
    }

    #[tokio::test]
    async fn test_fuzzy_bytes() {
        let backing = KVQSimpleCachedStore::new(Arc::new(KVQSimpleMemoryBackingStore::new()));
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![1, 2, 3], &vec![10]).unwrap();
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![1, 2, 5], &vec![20]).unwrap();
        <Arc<KVQSimpleMemoryBackingStore> as KVQBinaryStore>::set_ref(&backing.store, &vec![1, 3, 0], &vec![30]).unwrap();

        let store = KVQSimpleCachedStore::new(Arc::new(backing));

        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![1, 2, 4], 0).unwrap(),
            Some(vec![10])
        );
        assert_eq!(
            <KVQSimpleCachedStore<_> as KVQBinaryStore>::get_leq(&store, &vec![1, 2, 4], 1).unwrap(),
            Some(vec![10])
        );
    }
}
