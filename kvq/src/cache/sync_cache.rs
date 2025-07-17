use std::collections::Bound::Included;
use std::collections::BTreeMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::cache::{CacheValueType, KVQBinaryStoreCachedTrait};
use crate::traits::{KVQBinaryStore, KVQPair};

#[derive(Clone)]
pub struct KVQBinaryStoreCached<S: KVQBinaryStore> {
    pub store: Arc<S>,
    pub map: Arc<RwLock<BTreeMap<Vec<u8>, CacheValueType>>>,
    pub proper_delete_return: bool,
}

impl<S: KVQBinaryStore> KVQBinaryStoreCached<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self {
            store,
            map: Arc::new(RwLock::new(BTreeMap::new())),
            proper_delete_return: false,
        }
    }
    
    fn key_matches_fuzzy(&self, key: &Vec<u8>, target: &Vec<u8>, fuzzy_bytes: usize) -> bool {
        if fuzzy_bytes == 0 {
            return true;
        }
        
        if key.len() != target.len() {
            return false;
        }
        
        let non_fuzzy_len = key.len() - fuzzy_bytes;
        key[..non_fuzzy_len] == target[..non_fuzzy_len]
    }
}
impl<S: KVQBinaryStore> KVQBinaryStoreCachedTrait for KVQBinaryStoreCached<S> {
    fn is_removed(&self, key: &Vec<u8>) -> bool {
        match self.map.read().get(key) {
            Some(v) => match v {
                CacheValueType::Bytes(_) => false,
                CacheValueType::Removed => true,
            },
            None => false,
        }
    }
    fn get_non_removed_keys(&self) -> Vec<Vec<u8>> {
        self.map.read()
            .iter()
            .filter(|x| match x.1 {
                CacheValueType::Bytes(_) => true,
                CacheValueType::Removed => false,
            })
            .map(|x| x.0.to_owned())
            .collect::<Vec<_>>()
    }
    fn get_removed_keys(&self) -> Vec<Vec<u8>> {
        self.map.read()
            .iter()
            .filter(|x| match x.1 {
                CacheValueType::Bytes(_) => false,
                CacheValueType::Removed => true,
            })
            .map(|x| x.0.to_owned())
            .collect::<Vec<_>>()
    }
    fn flush_changes(&self) -> anyhow::Result<(Vec<KVQPair<Vec<u8>, Vec<u8>>>, Vec<Vec<u8>>)> {
        let mut map = self.map.write();
        let keys_to_set: Vec<KVQPair<&Vec<u8>, &Vec<u8>>> = map.iter().filter(|(_, vt)|{
            match vt {
                CacheValueType::Bytes(_) => true,
                CacheValueType::Removed => false,
            }
        }).map(|(k, vt)|{
            match vt {
                CacheValueType::Bytes(b) => Ok(KVQPair{
                    key: k,
                    value: b,
                }),
                CacheValueType::Removed => Err(anyhow::anyhow!("Cannot flush changes with removed keys")),
            }
        }).collect::<anyhow::Result<Vec<_>>>()?;

        let removed_keys = map
            .iter()
            .filter(|x| match x.1 {
                CacheValueType::Bytes(_) => false,
                CacheValueType::Removed => true,
            })
            .map(|x| x.0.to_owned())
            .collect::<Vec<_>>();

        let set_keys = keys_to_set.iter().map(|x| KVQPair{
            key: x.key.to_owned(),
            value: x.value.to_owned(),
        }).collect::<Vec<_>>();

        map.clear();
        Ok((set_keys, removed_keys))
    }

    fn clear_cache(&self) {
        self.map.write().clear();
    }

    fn flush_simple(&self) -> anyhow::Result<()> {
        let (keys_to_set, removed_keys) = {
            let mut map = self.map.write();
            let keys_to_set: Vec<KVQPair<Vec<u8>, Vec<u8>>> = map.iter().filter(|(_, vt)|{
                match vt {
                    CacheValueType::Bytes(_) => true,
                    CacheValueType::Removed => false,
                }
            }).map(|(k, vt)|{
                match vt {
                    CacheValueType::Bytes(b) => Ok(KVQPair{
                        key: k.clone(),
                        value: b.clone(),
                    }),
                    CacheValueType::Removed => Err(anyhow::anyhow!("Cannot flush changes with removed keys")),
                }
            }).collect::<anyhow::Result<Vec<_>>>()?;

            let removed_keys = map
                .iter()
                .filter(|x| match x.1 {
                    CacheValueType::Bytes(_) => false,
                    CacheValueType::Removed => true,
                })
                .map(|x| x.0.to_owned())
                .collect::<Vec<_>>();

            (keys_to_set, removed_keys)
        };

        // Convert Vec<KVQPair<Vec<u8>, Vec<u8>>> to Vec<KVQPair<&Vec<u8>, &Vec<u8>>>
        let keys_to_set_ref: Vec<KVQPair<&Vec<u8>, &Vec<u8>>> = keys_to_set
            .iter()
            .map(|kv| KVQPair {
                key: &kv.key,
                value: &kv.value,
            })
            .collect();

        self.store.set_many_ref(&keys_to_set_ref)?;
        self.store.delete_many(&removed_keys)?;

        self.map.write().clear();
        Ok(())
    }
}

impl<S: KVQBinaryStore> KVQBinaryStore for KVQBinaryStoreCached<S> {
    fn get_exact(&self, key: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
        match self.map.read().get(key) {
            Some(v) => match v {
                CacheValueType::Bytes(b) => Ok(b.to_owned()),
                CacheValueType::Removed => anyhow::bail!("Key {} not found", hex::encode(&key)),
            },
            None => self.store.get_exact(key),
        }
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<Vec<u8>>> {
        let mut result = Vec::new();
        for key in keys {
            let r = self.get_exact(key)?;
            result.push(r);
        }
        Ok(result)
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> anyhow::Result<Option<Vec<u8>>> {
        if fuzzy_bytes > key.len() {
            return Err(anyhow::anyhow!(
                "Fuzzy bytes must be less than or equal to key length"
            ));
        }

        let map = self.map.read();
        let mut cache_candidate: Option<KVQPair<Vec<u8>, Vec<u8>>> = None;
        
        for (k, v) in map.range(..=key.clone()) {
            match v {
                CacheValueType::Bytes(b) => {
                    if fuzzy_bytes == 0 || self.key_matches_fuzzy(k, key, fuzzy_bytes) {
                        cache_candidate = Some(KVQPair {
                            key: k.clone(),
                            value: b.clone(),
                        });
                    }
                }
                CacheValueType::Removed => {
                    if cache_candidate.as_ref().map_or(false, |c| c.key == *k) {
                        cache_candidate = None;
                    }
                }
            }
        }
        drop(map);
        
        let store_candidate = self.store.get_leq_kv(key, fuzzy_bytes)?;
        
        match (cache_candidate, store_candidate) {
            (None, None) => Ok(None),
            (Some(c), None) => Ok(Some(c.value)),
            (None, Some(s)) => {
                if self.map.read().get(&s.key).map_or(false, |v| matches!(v, CacheValueType::Removed)) {
                    Ok(None)
                } else {
                    Ok(Some(s.value))
                }
            }
            (Some(c), Some(s)) => {
                if self.map.read().get(&s.key).map_or(false, |v| matches!(v, CacheValueType::Removed)) {
                    Ok(Some(c.value))
                } else {
                    Ok(Some(if c.key >= s.key { c.value } else { s.value }))
                }
            }
        }
    }

    fn get_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        if fuzzy_bytes > key.len() {
            return Err(anyhow::anyhow!(
                "Fuzzy bytes must be less than or equal to key length"
            ));
        }

        let map = self.map.read();
        let mut cache_candidate: Option<KVQPair<Vec<u8>, Vec<u8>>> = None;
        
        for (k, v) in map.range(..=key.clone()) {
            match v {
                CacheValueType::Bytes(b) => {
                    if fuzzy_bytes == 0 || self.key_matches_fuzzy(k, key, fuzzy_bytes) {
                        cache_candidate = Some(KVQPair {
                            key: k.clone(),
                            value: b.clone(),
                        });
                    }
                }
                CacheValueType::Removed => {
                    if cache_candidate.as_ref().map_or(false, |c| c.key == *k) {
                        cache_candidate = None;
                    }
                }
            }
        }
        drop(map);
        
        let store_candidate = self.store.get_leq_kv(key, fuzzy_bytes)?;
        
        match (cache_candidate, store_candidate) {
            (None, None) => Ok(None),
            (Some(c), None) => Ok(Some(c)),
            (None, Some(s)) => {
                if self.map.read().get(&s.key).map_or(false, |v| matches!(v, CacheValueType::Removed)) {
                    Ok(None)
                } else {
                    Ok(Some(s))
                }
            }
            (Some(c), Some(s)) => {
                if self.map.read().get(&s.key).map_or(false, |v| matches!(v, CacheValueType::Removed)) {
                    Ok(Some(c))
                } else {
                    Ok(Some(if c.key >= s.key { c } else { s }))
                }
            }
        }
    }

    fn get_many_leq(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        let mut results: Vec<Option<Vec<u8>>> = Vec::with_capacity(keys.len());
        for k in keys {
            let r = self.get_leq(k, fuzzy_bytes)?;
            results.push(r.to_owned());
        }
        Ok(results)
    }

    fn get_many_leq_kv(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        let mut results: Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>> = Vec::with_capacity(keys.len());
        for k in keys {
            let r = self.get_leq_kv(k, fuzzy_bytes)?;
            results.push(r);
        }
        Ok(results)
    }

    fn get_exact_if_exists(&self, key: &Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        match {self.map.read().get(key)} {
            Some(v) => match v {
                CacheValueType::Bytes(b) => Ok(Some(b.to_owned())),
                CacheValueType::Removed => Ok(None),
            },
            None => self.store.get_exact_if_exists(key),
        }
    }
    fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> anyhow::Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        let key_end = key.to_vec();
        let mut base_key = key.to_vec();
        let key_len = base_key.len();
        if fuzzy_bytes > key_len {
            return Err(anyhow::anyhow!(
                "Fuzzy bytes must be less than or equal to key length"
            ));
        }

        for i in 0..fuzzy_bytes {
            base_key[key_len - i - 1] = 0;
        }

        let map = self.map.read();
        Ok(map
            .range((Included(base_key), Included(key_end)))
            .filter(|x| match x.1 {
                CacheValueType::Bytes(_) => true,
                CacheValueType::Removed => false,
            })
            .map(|(k, v)| KVQPair {
                key: k.to_owned(),
                value: match v {
                    CacheValueType::Bytes(b) => b.to_owned(),
                    CacheValueType::Removed => Vec::new(),
                },
            })
            .chain(
                self.store
                    .get_fuzzy_range_leq_kv(key, fuzzy_bytes)?
                    .into_iter()
                    .filter(|x| !map.contains_key(&x.key)),
            )
            .collect::<Vec<_>>())
    }

    // Write operations
    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        self.map.write().insert(key, CacheValueType::Bytes(value));
        Ok(())
    }

    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> anyhow::Result<()> {
        self.map.write()
            .insert(key.clone(), CacheValueType::Bytes(value.clone()));
        Ok(())
    }

    fn set_many_ref<'a>(
        &self,
        items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>],
    ) -> anyhow::Result<()> {
        let mut map = self.map.write();
        for item in items {
            map.insert(item.key.clone(), CacheValueType::Bytes(item.value.clone()));
        }
        Ok(())
    }

    fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> anyhow::Result<()> {
        let mut map = self.map.write();
        for item in items {
            map.insert(item.key, CacheValueType::Bytes(item.value));
        }
        Ok(())
    }

    fn delete(&self, key: &Vec<u8>) -> anyhow::Result<bool> {
        let r = {self.map.write().insert(key.clone(), CacheValueType::Removed)};
        if r.is_none() {
            if self.proper_delete_return {
                let r1 = self.get_exact_if_exists(key)?;
                if r1.is_some() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        } else {
            Ok(true)
        }
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> anyhow::Result<Vec<bool>> {
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            let r = self.delete(key)?;
            result.push(r);
        }
        Ok(result)
    }

    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> anyhow::Result<()> {
        if keys.len() != values.len() {
            anyhow::bail!("Keys and values must have the same length");
        } else {
            let mut map = self.map.write();
            for i in 0..keys.len() {
                map.insert(keys[i].clone(), CacheValueType::Bytes(values[i].clone()));
            }
            Ok(())
        }
    }
}