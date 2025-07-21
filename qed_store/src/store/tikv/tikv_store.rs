use anyhow::Result;
use async_trait::async_trait;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};
use std::{fmt::Debug, sync::Arc};
use tikv_client::{Key, Transaction, TransactionClient, Value};
use tikv_client::proto::kvrpcpb::{Mutation, Op};
use super::config::TiKVConfig;

// Maximum number of entries to scan in a single operation
const MAX_SCAN_ENTRIES: u32 = 1000;

#[derive(Clone)]
pub struct TiKVStore {
    connection: Arc<TransactionClient>,
    namespace_bytes: Vec<u8>,
    config: TiKVConfig,
}

impl Debug for TiKVStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TiKVStore {{ connection: {:?}, namespace: {:?} }}", self.config.pd_endpoints, self.config.namespace)
    }
}

impl TiKVStore {
    pub async fn new(config: TiKVConfig) -> Result<Self> {
        let connection = TransactionClient::new(config.pd_endpoints.clone()).await?;
        let namespace_bytes = config.namespace.as_bytes().to_vec();
        
        Ok(Self {
            connection: Arc::from(connection),
            namespace_bytes,
            config,
        })
    }

    fn make_key(&self, key: &[u8]) -> Key {
        let mut full_key = self.namespace_bytes.clone();
        full_key.extend_from_slice(key);
        Key::from(full_key)
    }

    // Helper method to create scan range with namespace
    fn make_scan_range(&self, start_key: &[u8], end_key: &[u8]) -> (Key, Key) {
        let mut scan_start = self.namespace_bytes.clone();
        scan_start.extend_from_slice(start_key);
        
        let mut scan_end = self.namespace_bytes.clone();
        scan_end.extend_from_slice(end_key);
        
        (Key::from(scan_start), Key::from(scan_end))
    }

    // Execute read-only transaction without commit/rollback management
    async fn execute_read_transaction<F, Fut, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(Transaction) -> Fut,
        Fut: std::future::Future<Output = Result<R>>,
    {
        let txn = self.connection.begin_optimistic().await?;
        f(txn).await
    }

    // Execute batch operations with automatic commit/rollback
    async fn execute_write_transaction<F, Fut>(&self, f: F) -> Result<()>
    where
        F: FnOnce(Transaction) -> Fut,
        Fut: std::future::Future<Output = Result<Transaction>>,
    {
        let txn = self.connection.begin_optimistic().await?;
        match f(txn).await {
            Ok(mut txn) => {
                txn.commit().await?;
                Ok(())
            }
            Err(e) => {
                Err(e)
            }
        }
    }
}

#[async_trait]
impl KVQBinaryStoreAsync for TiKVStore {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> Result<Option<Vec<u8>>> {
        let tikv_key = self.make_key(key);
        
        self.execute_read_transaction(|mut txn| async move {
            let value = txn.get(tikv_key).await?;
            Ok(value.map(|v| v.to_vec()))
        }).await
    }

    async fn get_exact(&self, key: &Vec<u8>) -> Result<Vec<u8>> {
        match KVQBinaryStoreAsync::get_exact_if_exists(self, key).await? {
            Some(value) => Ok(value),
            None => Err(anyhow::anyhow!("Key not found")),
        }
    }

    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> Result<Vec<Vec<u8>>> {
        let namespace_bytes = self.namespace_bytes.clone();
        
        self.execute_read_transaction(|mut txn| async move {
            let tikv_keys: Vec<Key> = keys.iter().map(|key| {
                let mut full_key = namespace_bytes.clone();
                full_key.extend_from_slice(key);
                Key::from(full_key)
            }).collect();

            let batch_result = txn.batch_get(tikv_keys.clone()).await?;
            
            // Collect batch_result into a Vec for reuse
            let batch_vec: Vec<_> = batch_result.collect();
            
            // Use Vec instead of HashMap for better performance with small datasets
            let mut results = Vec::with_capacity(keys.len());
            for tikv_key in tikv_keys {
                let value = batch_vec.iter()
                    .find(|kv| kv.key() == &tikv_key)
                    .map(|kv| kv.value().to_vec())
                    .unwrap_or_default();
                results.push(value);
            }
            
            Ok(results)
        }).await
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> Result<Option<Vec<u8>>> {
        let namespace_bytes = &self.namespace_bytes;
        
        self.execute_read_transaction(|mut txn| async move {
            // First try exact match
            let exact_key = {
                let mut full_key = Vec::with_capacity(namespace_bytes.len() + key.len());
                full_key.extend_from_slice(namespace_bytes);
                full_key.extend_from_slice(key);
                Key::from(full_key)
            };
            
            if let Some(value) = txn.get(exact_key).await? {
                return Ok(Some(value.to_vec()));
            }
            
            // If no exact match and fuzzy_bytes == 0, return None
            if fuzzy_bytes == 0 {
                return Ok(None);
            }
            
            // Calculate optimized scan range
            let base_key_len = key.len().saturating_sub(fuzzy_bytes);
            let base_key = &key[..base_key_len];
            
            // Create tighter scan range for better performance
            let (scan_start, scan_end) = {
                let mut start = Vec::with_capacity(namespace_bytes.len() + base_key.len());
                start.extend_from_slice(namespace_bytes);
                start.extend_from_slice(base_key);
                
                let mut end = Vec::with_capacity(namespace_bytes.len() + key.len() + 1);
                end.extend_from_slice(namespace_bytes);
                end.extend_from_slice(key);
                end.push(0x00); // Use 0x00 instead of 0xFF for tighter bound
                
                (Key::from(start), Key::from(end))
            };
            
            // Use reverse scan for better LEQ performance
            let scan_result = txn.scan_reverse(scan_end..scan_start, MAX_SCAN_ENTRIES).await?;
            let namespace_len = namespace_bytes.len();
            
            // Find the first (largest) key that is <= target key
            for kv_pair in scan_result {
                let actual_key: Vec<u8> = kv_pair.key().clone().into();
                
                // Fast namespace check with length validation
                if actual_key.len() <= namespace_len {
                    continue;
                }
                
                // Extract key without namespace (avoid allocation)
                let key_without_ns = &actual_key[namespace_len..];
                
                // Check if this key is <= target key
                if key_without_ns <= key.as_slice() {
                    return Ok(Some(kv_pair.value().to_vec()));
                }
            }
            
            Ok(None)
        }).await
    }

    async fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        let namespace_bytes = &self.namespace_bytes;
        
        self.execute_read_transaction(|mut txn| async move {
            // Calculate optimized scan range
            let base_key_len = key.len().saturating_sub(fuzzy_bytes);
            let base_key = &key[..base_key_len];
            
            // Create tighter scan range
            let (scan_start, scan_end) = {
                let mut start = Vec::with_capacity(namespace_bytes.len() + base_key.len());
                start.extend_from_slice(namespace_bytes);
                start.extend_from_slice(base_key);
                
                let mut end = Vec::with_capacity(namespace_bytes.len() + key.len() + 1);
                end.extend_from_slice(namespace_bytes);
                end.extend_from_slice(key);
                end.push(0x00);
                
                (Key::from(start), Key::from(end))
            };
            
            // Pre-allocate results vector with estimated capacity
            let mut results = Vec::with_capacity(64);
            let namespace_len = namespace_bytes.len();
            
            // Use forward scan and collect all matching results
            let scan_result = txn.scan(scan_start..scan_end, MAX_SCAN_ENTRIES).await?;
            
            for kv_pair in scan_result {
                let actual_key: Vec<u8> = kv_pair.key().clone().into();
                
                // Fast namespace validation
                if actual_key.len() <= namespace_len {
                    continue;
                }
                
                // Extract key without namespace (avoid unnecessary allocation)
                let key_without_ns = &actual_key[namespace_len..];
                
                // Check if this key is <= target key
                if key_without_ns <= key.as_slice() {
                    results.push(KVQPair {
                        key: key_without_ns.to_vec(),
                        value: kv_pair.value().to_vec(),
                    });
                }
            }
            
            // Results are already sorted due to TiKV's key ordering
            Ok(results)
        }).await
    }

    async fn get_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        let namespace_bytes = &self.namespace_bytes;
        
        self.execute_read_transaction(|mut txn| async move {
            // First try exact match
            let exact_key = {
                let mut full_key = Vec::with_capacity(namespace_bytes.len() + key.len());
                full_key.extend_from_slice(namespace_bytes);
                full_key.extend_from_slice(key);
                Key::from(full_key)
            };
            
            if let Some(value) = txn.get(exact_key).await? {
                return Ok(Some(KVQPair {
                    key: key.clone(),
                    value: value.to_vec(),
                }));
            }
            
            // If no exact match and fuzzy_bytes == 0, return None
            if fuzzy_bytes == 0 {
                return Ok(None);
            }
            
            // Calculate optimized scan range
            let base_key_len = key.len().saturating_sub(fuzzy_bytes);
            let base_key = &key[..base_key_len];
            
            // Create tighter scan range
            let (scan_start, scan_end) = {
                let mut start = Vec::with_capacity(namespace_bytes.len() + base_key.len());
                start.extend_from_slice(namespace_bytes);
                start.extend_from_slice(base_key);
                
                let mut end = Vec::with_capacity(namespace_bytes.len() + key.len() + 1);
                end.extend_from_slice(namespace_bytes);
                end.extend_from_slice(key);
                end.push(0x00);
                
                (Key::from(start), Key::from(end))
            };
            
            // Use reverse scan for optimal LEQ performance
            let scan_result = txn.scan_reverse(scan_end..scan_start, MAX_SCAN_ENTRIES).await?;
            let namespace_len = namespace_bytes.len();
            
            // Find the first (largest) key that is <= target key
            for kv_pair in scan_result {
                let actual_key: Vec<u8> = kv_pair.key().clone().into();
                
                // Fast namespace validation
                if actual_key.len() <= namespace_len {
                    continue;
                }
                
                // Extract key without namespace (avoid allocation)
                let key_without_ns = &actual_key[namespace_len..];
                
                // Check if this key is <= target key
                if key_without_ns <= key.as_slice() {
                    return Ok(Some(KVQPair {
                        key: key_without_ns.to_vec(),
                        value: kv_pair.value().to_vec(),
                    }));
                }
            }
            
            Ok(None)
        }).await
    }

    async fn get_many_leq(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> Result<Vec<Option<Vec<u8>>>> {
        // Optimized batch implementation using single transaction
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        
        let namespace_bytes = &self.namespace_bytes;
        
        self.execute_read_transaction(|mut txn| async move {
            let mut results = Vec::with_capacity(keys.len());
            
            // First, try exact matches in batch
            let exact_keys: Vec<Key> = keys.iter().map(|key| {
                let mut full_key = Vec::with_capacity(namespace_bytes.len() + key.len());
                full_key.extend_from_slice(namespace_bytes);
                full_key.extend_from_slice(key);
                Key::from(full_key)
            }).collect();
            
            let batch_result = txn.batch_get(exact_keys).await?;
            let exact_matches: std::collections::HashMap<Vec<u8>, Vec<u8>> = batch_result
                .map(|kv| (kv.key().clone().into(), kv.value().to_vec()))
                .collect();
            
            // Process each key
            for key in keys {
                // Check exact match first
                let exact_key_bytes = {
                    let mut full_key = Vec::with_capacity(namespace_bytes.len() + key.len());
                    full_key.extend_from_slice(namespace_bytes);
                    full_key.extend_from_slice(key);
                    full_key
                };
                
                if let Some(value) = exact_matches.get(&exact_key_bytes) {
                    results.push(Some(value.clone()));
                    continue;
                }
                
                // If no exact match and fuzzy_bytes == 0, push None
                if fuzzy_bytes == 0 {
                    results.push(None);
                    continue;
                }
                
                // Perform fuzzy search for this key
                let base_key_len = key.len().saturating_sub(fuzzy_bytes);
                let base_key = &key[..base_key_len];
                
                let (scan_start, scan_end) = {
                    let mut start = Vec::with_capacity(namespace_bytes.len() + base_key.len());
                    start.extend_from_slice(namespace_bytes);
                    start.extend_from_slice(base_key);
                    
                    let mut end = Vec::with_capacity(namespace_bytes.len() + key.len() + 1);
                    end.extend_from_slice(namespace_bytes);
                    end.extend_from_slice(key);
                    end.push(0x00);
                    
                    (Key::from(start), Key::from(end))
                };
                
                // Use smaller scan limit for batch operations
                let scan_result = txn.scan_reverse(scan_end..scan_start, 100).await?;
                let namespace_len = namespace_bytes.len();
                
                let mut found = false;
                for kv_pair in scan_result {
                    let actual_key: Vec<u8> = kv_pair.key().clone().into();
                    
                    if actual_key.len() <= namespace_len {
                        continue;
                    }
                    
                    let key_without_ns = &actual_key[namespace_len..];
                    
                    if key_without_ns <= key.as_slice() {
                        results.push(Some(kv_pair.value().to_vec()));
                        found = true;
                        break;
                    }
                }
                
                if !found {
                    results.push(None);
                }
            }
            
            Ok(results)
        }).await
    }

    async fn get_many_leq_kv(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        
        let namespace_bytes = &self.namespace_bytes;
        
        self.execute_read_transaction(|mut txn| async move {
            let mut results = Vec::with_capacity(keys.len());
            
            // First, try exact matches in batch
            let exact_keys: Vec<Key> = keys.iter().map(|key| {
                let mut full_key = Vec::with_capacity(namespace_bytes.len() + key.len());
                full_key.extend_from_slice(namespace_bytes);
                full_key.extend_from_slice(key);
                Key::from(full_key)
            }).collect();
            
            let batch_result = txn.batch_get(exact_keys).await?;
            let exact_matches: std::collections::HashMap<Vec<u8>, Vec<u8>> = batch_result
                .map(|kv| (kv.key().clone().into(), kv.value().to_vec()))
                .collect();
            
            // Process each key
            for key in keys {
                // Check exact match first
                let exact_key_bytes = {
                    let mut full_key = Vec::with_capacity(namespace_bytes.len() + key.len());
                    full_key.extend_from_slice(namespace_bytes);
                    full_key.extend_from_slice(key);
                    full_key
                };
                
                if let Some(value) = exact_matches.get(&exact_key_bytes) {
                    results.push(Some(KVQPair {
                        key: key.clone(),
                        value: value.clone(),
                    }));
                    continue;
                }
                
                // If no exact match and fuzzy_bytes == 0, push None
                if fuzzy_bytes == 0 {
                    results.push(None);
                    continue;
                }
                
                // Perform fuzzy search for this key
                let base_key_len = key.len().saturating_sub(fuzzy_bytes);
                let base_key = &key[..base_key_len];
                
                let (scan_start, scan_end) = {
                    let mut start = Vec::with_capacity(namespace_bytes.len() + base_key.len());
                    start.extend_from_slice(namespace_bytes);
                    start.extend_from_slice(base_key);
                    
                    let mut end = Vec::with_capacity(namespace_bytes.len() + key.len() + 1);
                    end.extend_from_slice(namespace_bytes);
                    end.extend_from_slice(key);
                    end.push(0x00);
                    
                    (Key::from(start), Key::from(end))
                };
                
                // Use smaller scan limit and reverse scan for batch operations
                let scan_result = txn.scan_reverse(scan_end..scan_start, MAX_SCAN_ENTRIES).await?;
                let namespace_len = namespace_bytes.len();
                
                let mut found = false;
                for kv_pair in scan_result {
                    let actual_key: Vec<u8> = kv_pair.key().clone().into();
                    
                    if actual_key.len() <= namespace_len {
                        continue;
                    }
                    
                    let key_without_ns = &actual_key[namespace_len..];
                    
                    if key_without_ns <= key.as_slice() {
                        results.push(Some(KVQPair {
                            key: key_without_ns.to_vec(),
                            value: kv_pair.value().to_vec(),
                        }));
                        found = true;
                        break;
                    }
                }
                
                if !found {
                    results.push(None);
                }
            }
            
            Ok(results)
        }).await
    }

    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let tikv_key = self.make_key(&key);
        let tikv_value = Value::from(value);
        
        self.execute_write_transaction(|mut txn| async move {
            txn.put(tikv_key, tikv_value).await?;
            Ok(txn)
        }).await
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> Result<()> {
        KVQBinaryStoreAsync::set(self, key.clone(), value.clone()).await
    }

    async fn set_many_ref<'a>(
        &self,
        items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>],
    ) -> Result<()> {
        let namespace_bytes = self.namespace_bytes.clone();
        
        self.execute_write_transaction(|mut txn| async move {
            let mutations: Vec<Mutation> = items
                .iter()
                .map(|item| {
                    let mut full_key = namespace_bytes.clone();
                    full_key.extend_from_slice(item.key);
                    Mutation {
                        op: Op::Put.into(),
                        key: full_key,
                        value: item.value.to_vec(),
                        ..Default::default()
                    }
                })
                .collect();

            txn.batch_mutate(mutations).await?;
            Ok(txn)
        }).await
    }

    async fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> Result<()> {
        let namespace_bytes = self.namespace_bytes.clone();
        
        self.execute_write_transaction(|mut txn| async move {
            let mutations: Vec<Mutation> = items
                .into_iter()
                .map(|item| {
                    let mut full_key = namespace_bytes.clone();
                    full_key.extend_from_slice(&item.key);
                    Mutation {
                        op: Op::Put.into(),
                        key: full_key,
                        value: item.value,
                        ..Default::default()
                    }
                })
                .collect();
            txn.batch_mutate(mutations).await?;
            Ok(txn)
        }).await
    }

    async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> Result<()> {
        if keys.len() != values.len() {
            return Err(anyhow::anyhow!("Keys and values must have the same length"));
        }

        let namespace_bytes = self.namespace_bytes.clone();
        
        self.execute_write_transaction(|mut txn| async move {
            let mutations: Vec<Mutation> = keys
                .iter()
                .zip(values.iter())
                .map(|(key, value)| {
                    let mut full_key = namespace_bytes.clone();
                    full_key.extend_from_slice(key);
                    Mutation {
                        op: Op::Put.into(),
                        key: full_key,
                        value: value.clone(),
                        ..Default::default()
                    }
                })
                .collect();
            txn.batch_mutate(mutations).await?;
            Ok(txn)
        }).await
    }

    async fn delete(&self, key: &Vec<u8>) -> Result<bool> {
        let tikv_key = self.make_key(key);
        self.execute_write_transaction(|mut txn| async move {
            txn.delete(tikv_key).await?;
            Ok(txn)
        }).await?;
        Ok(true)
    }

    async fn delete_many(&self, keys: &[Vec<u8>]) -> Result<Vec<bool>> {
        let namespace_bytes = self.namespace_bytes.clone();
        
        self.execute_write_transaction(|mut txn| async move {
            let mutations: Vec<Mutation> = keys
                .iter()
                .map(|key| {
                    let mut full_key = namespace_bytes.clone();
                    full_key.extend_from_slice(key);
                    Mutation {
                        op: Op::Del.into(),
                        key: full_key,
                        value: vec![],
                        ..Default::default()
                    }
                })
                .collect();
            txn.batch_mutate(mutations).await?;
            Ok(txn)
        }).await?;
        Ok(vec![true; keys.len()].into_iter().collect::<Vec<bool>>())
    }

    async fn set_and_delete_many(
        &self,
        keys_to_set: &[KVQPair<&Vec<u8>, &Vec<u8>>],
        keys_to_delete: &[Vec<u8>],
    ) -> Result<()> {
        let namespace_bytes = self.namespace_bytes.clone();
        
        self.execute_write_transaction(|mut txn| async move {
            let mut mutations = Vec::new();
            for item in keys_to_set {
                let mut full_key = namespace_bytes.clone();
                full_key.extend_from_slice(item.key);
                mutations.push(Mutation {
                    op: Op::Put.into(),
                    key: full_key,
                    value: item.value.to_vec(),
                    ..Default::default()
                });
            }
            
            for key in keys_to_delete {
                let mut full_key = namespace_bytes.clone();
                full_key.extend_from_slice(key);
                mutations.push(Mutation {
                    op: Op::Del.into(),
                    key: full_key,
                    value: vec![],
                    ..Default::default()
                });
            }
            
            if !mutations.is_empty() {
                txn.batch_mutate(mutations).await?;
            }
            Ok(txn)
        }).await
    }
}

impl KVQBinaryStore for TiKVStore {
    fn get_exact_if_exists(&self, key: &Vec<u8>) -> Result<Option<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::get_exact_if_exists(self, key).await
            })
        })
    }

    fn get_exact(&self, key: &Vec<u8>) -> Result<Vec<u8>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::get_exact(self, key).await
            })
        })
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> Result<Vec<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::get_many_exact(self, keys).await
            })
        })
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> Result<Option<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::get_leq(self, key, fuzzy_bytes).await
            })
        })
    }

    fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::get_fuzzy_range_leq_kv(self, key, fuzzy_bytes).await
            })
        })
    }

    fn get_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::get_leq_kv(self, key, fuzzy_bytes).await
            })
        })
    }

    fn get_many_leq(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> Result<Vec<Option<Vec<u8>>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::get_many_leq(self, keys, fuzzy_bytes).await
            })
        })
    }

    fn get_many_leq_kv(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::get_many_leq_kv(self, keys, fuzzy_bytes).await
            })
        })
    }

    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::set(self, key, value).await
            })
        })
    }

    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::set_ref(self, key, value).await
            })
        })
    }

    fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::set_many_ref(self, items).await
            })
        })
    }

    fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::set_many_vec(self, items).await
            })
        })
    }

    fn delete(&self, key: &Vec<u8>) -> Result<bool> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::delete(self, key).await
            })
        })
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> Result<Vec<bool>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::delete_many(self, keys).await
            })
        })
    }

    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::set_many_split_ref(self, keys, values).await
            })
        })
    }

    fn set_and_delete_many(
        &self,
        keys_to_set: &[KVQPair<&Vec<u8>, &Vec<u8>>],
        keys_to_delete: &[Vec<u8>],
    ) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KVQBinaryStoreAsync::set_and_delete_many(self, keys_to_set, keys_to_delete).await
            })
        })
    }
}
