use super::config::TiKVConfig;
use anyhow::Result;
use async_trait::async_trait;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};
use std::{fmt::Debug, sync::Arc};
use tikv_client::proto::kvrpcpb::{Mutation, Op};
use tikv_client::{CheckLevel, Key, Snapshot, Transaction, TransactionClient, TransactionOptions, Value};

// Maximum number of entries to scan in a single operation
const MAX_SCAN_ENTRIES: u32 = 1000;

#[derive(Clone)]
pub struct TiKVStore {
    connection: Arc<TransactionClient>,
    namespace_bytes: Vec<u8>,
    config: TiKVConfig,
}

pub fn prefix_key(prefix: &[u8], key: &[u8]) -> Key {
    let mut full_key = prefix.to_vec();
    full_key.extend_from_slice(key);
    full_key.into()
}

impl Debug for TiKVStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TiKVStore {{ connection: {:?}, namespace: {:?} }}",
            self.config.pd_endpoints, self.config.namespace
        )
    }
}

impl TiKVStore {
    pub async fn new(config: TiKVConfig) -> Result<Self> {
        let pd_endpoints = config.get_pd_endpoints();
        let connection = TransactionClient::new(pd_endpoints).await?;
        let namespace_bytes = config.namespace.as_bytes().to_vec();

        Ok(Self {
            connection: Arc::from(connection),
            namespace_bytes,
            config,
        })
    }

    fn make_key(&self, key: &[u8]) -> Key {
        prefix_key(self.namespace_bytes.as_slice(), key)
    }

    // Helper method to create LEQ scan range
    fn make_leq_scan_range(&self, key: &[u8], fuzzy_bytes: usize) -> (Key, Key) {
        let base_key_len = key.len().saturating_sub(fuzzy_bytes);
        let base_key = &key[..base_key_len];

        let mut start = Vec::with_capacity(self.namespace_bytes.len() + base_key.len());
        start.extend_from_slice(&self.namespace_bytes);
        start.extend_from_slice(base_key);

        let mut end = Vec::with_capacity(self.namespace_bytes.len() + key.len() + 1);
        end.extend_from_slice(&self.namespace_bytes);
        end.extend_from_slice(key);
        end.push(0x00);

        (Key::from(start), Key::from(end))
    }

    fn extract_key_without_namespace<'a>(&self, full_key: &'a [u8]) -> Option<&'a [u8]> {
        if full_key.len() <= self.namespace_bytes.len() {
            None
        } else {
            Some(&full_key[self.namespace_bytes.len()..])
        }
    }

    async fn find_leq_scan_reverse(
        &self,
        snapshot: &mut Snapshot,
        key: &[u8],
        fuzzy_bytes: usize,
    ) -> Result<Option<tikv_client::KvPair>> {
        let (scan_start, mut scan_end) = self.make_leq_scan_range(key, fuzzy_bytes);
        while scan_start < scan_end {
            let scan_result = snapshot
                .scan_reverse(scan_start.clone()..scan_end.clone(), MAX_SCAN_ENTRIES)
                .await?
                .collect::<Vec<_>>();
            if scan_result.is_empty() {
                break;
            }

            for kv_pair in scan_result {
                let actual_key: Vec<u8> = kv_pair.key().clone().into();

                if let Some(key_without_ns) = self.extract_key_without_namespace(&actual_key) {
                    if key_without_ns <= key {
                        return Ok(Some(kv_pair));
                    }
                    scan_end = actual_key.into();
                }
            }
        }
        Ok(None)
    }

    async fn begin_pessimistic(&self) -> tikv_client::Result<Transaction> {
        let new_pessimistic = TransactionOptions::new_pessimistic();
        let new_pessimistic = new_pessimistic.drop_check(CheckLevel::Warn);
        self.connection.begin_with_options(new_pessimistic).await
    }

    async fn begin_optimistic(&self) -> tikv_client::Result<Transaction> {
        let new_optimistic = TransactionOptions::new_optimistic();
        let new_optimistic = new_optimistic.drop_check(CheckLevel::Warn);
        self.connection.begin_with_options(new_optimistic).await
    }

    async fn snapshot(&self) -> tikv_client::Result<Snapshot>{
        let options = TransactionOptions::default();
        let options = options.drop_check(CheckLevel::Warn);
        Ok(self.connection.snapshot(
            self.connection.current_timestamp().await?,
            options,
        ))
    }

    async fn with_read_txn<F, Fut, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(Transaction) -> Fut + Send,
        Fut: std::future::Future<Output = Result<(R, Transaction)>> + Send,
    {
        let txn = self.begin_optimistic().await?;
        let (result, _) = f(txn).await?;
        Ok(result)
    }

    async fn with_snapshot<F, Fut, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(Snapshot) -> Fut,
        Fut: std::future::Future<Output = Result<R>>,
    {
        f(self.snapshot().await?).await
    }

    async fn with_pessimistic_txn<F, Fut>(&self, f: F) -> Result<()>
    where
        F: FnOnce(Transaction) -> Fut,
        Fut: std::future::Future<Output = Result<Transaction>>,
    {
        let txn = self.begin_pessimistic().await?;
        self.with_txn(txn, f).await

    }

    async fn with_optimistic_txn<F, Fut>(&self, f: F) -> Result<()>
    where
        F: FnOnce(Transaction) -> Fut,
        Fut: std::future::Future<Output = Result<Transaction>>,
    {
        let txn = self.begin_optimistic().await?;
        self.with_txn(txn, f).await
    }


    async fn with_txn<F, Fut>(&self, txn: Transaction, f: F) -> Result<()>
    where
        F: FnOnce(Transaction) -> Fut,
        Fut: std::future::Future<Output = Result<Transaction>>,
    {
        let mut txn = f(txn).await?;
        match txn.commit().await {
            Ok(_) => Ok(()),
            Err(commit_err) => {
                // Commit failed, need to rollback
                if let Err(rollback_err) = txn.rollback().await {
                    // Log rollback error but return original commit error
                    eprintln!(
                        "Warning: Failed to rollback transaction after commit failure: {}",
                        rollback_err
                    );
                }
                Err(anyhow::anyhow!(commit_err))
            }
        }
    }
}

#[async_trait]
impl KVQBinaryStoreAsync for TiKVStore {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> Result<Option<Vec<u8>>> {
        let tikv_key = self.make_key(key);

        self.with_snapshot(|mut txn| async move {
            let value = txn.get(tikv_key).await?;
            Ok(value.map(|v| v.to_vec()))
        })
        .await
    }

    async fn get_exact(&self, key: &Vec<u8>) -> Result<Vec<u8>> {
        match KVQBinaryStoreAsync::get_exact_if_exists(self, key).await? {
            Some(value) => Ok(value),
            None => Err(anyhow::anyhow!("Key not found")),
        }
    }

    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> Result<Vec<Vec<u8>>> {
        let namespace_bytes = self.namespace_bytes.clone();

        self.with_snapshot(|mut txn| async move {
            let tikv_keys: Vec<Key> = keys
                .iter()
                .map(|key| prefix_key(&namespace_bytes, key))
                .collect();
            let batch_result = txn.batch_get(tikv_keys.clone()).await?;
            // Collect batch_result into a Vec for reuse
            let batch_vec: Vec<_> = batch_result.collect();
            // Use Vec instead of HashMap for better performance with small datasets
            let mut results = Vec::with_capacity(keys.len());
            for tikv_key in tikv_keys {
                let value = batch_vec
                    .iter()
                    .find(|kv| kv.key() == &tikv_key)
                    .map(|kv| kv.value().to_vec())
                    .unwrap_or_default();
                results.push(value);
            }

            Ok(results)
        })
        .await
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> Result<Option<Vec<u8>>> {
        self.with_snapshot(|mut snapshot| async move {
            // First try exact match
            if let Some(value) = snapshot.get(self.make_key(key)).await? {
                return Ok(Some(value.to_vec()));
            }

            // Try fuzzy search if needed
            if let Some(kv_pair) = self
                .find_leq_scan_reverse(&mut snapshot, key, fuzzy_bytes)
                .await?
            {
                return Ok(Some(kv_pair.value().to_vec()));
            }

            Ok(None)
        })
        .await
    }

    async fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        self.with_snapshot(|mut snapshot| async move {
            let (mut scan_start, scan_end) = self.make_leq_scan_range(key, fuzzy_bytes);
            let mut results = Vec::with_capacity(64);
            while scan_start < scan_end {
                // Use forward scan and collect all matching results
                let scan_result = snapshot
                    .scan(scan_start.clone()..scan_end.clone(), MAX_SCAN_ENTRIES)
                    .await?
                    .collect::<Vec<_>>();
                if scan_result.is_empty() {
                    break;
                }

                for kv_pair in scan_result {
                    let actual_key: Vec<u8> = kv_pair.key().clone().into();

                    if let Some(key_without_ns) = self.extract_key_without_namespace(&actual_key) {
                        // Check if this key is <= target key
                        if key_without_ns <= key.as_slice() {
                            results.push(KVQPair {
                                key: key_without_ns.to_vec(),
                                value: kv_pair.value().to_vec(),
                            });
                        }
                        let mut next_key = actual_key;
                        next_key.push(0x00);
                        scan_start = next_key.into();
                    }
                }
            }
            // Results are already sorted due to TiKV's key ordering
            Ok(results)
        })
        .await
    }

    async fn get_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        self.with_snapshot(|mut snapshot| async move {
            // First try exact match
            if let Some(value) = snapshot.get(self.make_key(key)).await? {
                return Ok(Some(KVQPair {
                    key: key.clone(),
                    value: value.to_vec(),
                }));
            }

            // Try fuzzy search if needed
            if let Some(kv_pair) = self
                .find_leq_scan_reverse(&mut snapshot, key, fuzzy_bytes)
                .await?
            {
                if let Some(key_without_ns) =
                    self.extract_key_without_namespace(kv_pair.key().into())
                {
                    return Ok(Some(KVQPair {
                        key: key_without_ns.to_vec(),
                        value: kv_pair.value().to_vec(),
                    }));
                }
            }

            Ok(None)
        })
        .await
    }

    async fn get_many_leq(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> Result<Vec<Option<Vec<u8>>>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        self.with_snapshot(|mut snapshot| async move {
            let mut results = Vec::with_capacity(keys.len());

            // Process each key individually since we're using snapshot
            for key in keys {
                // First try exact match
                if let Some(value) = snapshot.get(self.make_key(key)).await? {
                    results.push(Some(value.to_vec()));
                    continue;
                }

                // Try fuzzy search if needed
                if let Some(kv_pair) = self
                    .find_leq_scan_reverse(&mut snapshot, key, fuzzy_bytes)
                    .await?
                {
                    results.push(Some(kv_pair.value().to_vec()));
                } else {
                    results.push(None);
                }
            }

            Ok(results)
        })
        .await
    }

    async fn get_many_leq_kv(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        self.with_snapshot(|mut snapshot| async move {
            let mut results = Vec::with_capacity(keys.len());

            // Process each key individually since we're using snapshot
            for key in keys {
                // First try exact match
                if let Some(value) = snapshot.get(self.make_key(key)).await? {
                    results.push(Some(KVQPair {
                        key: key.clone(),
                        value: value.to_vec(),
                    }));
                    continue;
                }

                // Try fuzzy search if needed
                if let Some(kv_pair) = self
                    .find_leq_scan_reverse(&mut snapshot, key, fuzzy_bytes)
                    .await?
                {
                    if let Some(key_without_ns) =
                        self.extract_key_without_namespace(kv_pair.key().into())
                    {
                        results.push(Some(KVQPair {
                            key: key_without_ns.to_vec(),
                            value: kv_pair.value().to_vec(),
                        }));
                    } else {
                        results.push(None);
                    }
                } else {
                    results.push(None);
                }
            }

            Ok(results)
        })
        .await
    }

    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let tikv_key = self.make_key(&key);
        let tikv_value = Value::from(value);

        self.with_optimistic_txn(|mut txn| async move {
            txn.put(tikv_key, tikv_value).await?;
            Ok(txn)
        })
        .await
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> Result<()> {
        KVQBinaryStoreAsync::set(self, key.clone(), value.clone()).await
    }

    async fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> Result<()> {
        let namespace_bytes = self.namespace_bytes.clone();

        self.with_optimistic_txn(|mut txn| async move {
            let mutations: Vec<Mutation> = items
                .iter()
                .map(|item| Mutation {
                    op: Op::Put.into(),
                    key: prefix_key(&namespace_bytes, item.key).into(),
                    value: item.value.to_vec(),
                    ..Default::default()
                })
                .collect();

            txn.batch_mutate(mutations).await?;
            Ok(txn)
        })
        .await
    }

    async fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> Result<()> {
        let namespace_bytes = self.namespace_bytes.clone();

        self.with_optimistic_txn(|mut txn| async move {
            let mutations: Vec<Mutation> = items
                .into_iter()
                .map(|item| Mutation {
                    op: Op::Put.into(),
                    key: prefix_key(&namespace_bytes, &item.key).into(),
                    value: item.value,
                    ..Default::default()
                })
                .collect();
            txn.batch_mutate(mutations).await?;
            Ok(txn)
        })
        .await
    }

    async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> Result<()> {
        if keys.len() != values.len() {
            return Err(anyhow::anyhow!("Keys and values must have the same length"));
        }

        let namespace_bytes = self.namespace_bytes.clone();

        self.with_optimistic_txn(|mut txn| async move {
            let mutations: Vec<Mutation> = keys
                .iter()
                .zip(values.iter())
                .map(|(key, value)| Mutation {
                    op: Op::Put.into(),
                    key: prefix_key(&namespace_bytes, key).into(),
                    value: value.clone(),
                    ..Default::default()
                })
                .collect();
            txn.batch_mutate(mutations).await?;
            Ok(txn)
        })
        .await
    }

    async fn delete(&self, key: &Vec<u8>) -> Result<bool> {
        let tikv_key = self.make_key(key);
        self.with_optimistic_txn(|mut txn| async move {
            txn.delete(tikv_key).await?;
            Ok(txn)
        })
        .await?;
        Ok(true)
    }

    async fn delete_many(&self, keys: &[Vec<u8>]) -> Result<Vec<bool>> {
        let namespace_bytes = self.namespace_bytes.clone();

        self.with_optimistic_txn(|mut txn| async move {
            let mutations: Vec<Mutation> = keys
                .iter()
                .map(|key| Mutation {
                    op: Op::Del.into(),
                    key: prefix_key(&namespace_bytes, key).into(),
                    value: vec![],
                    ..Default::default()
                })
                .collect();
            txn.batch_mutate(mutations).await?;
            Ok(txn)
        })
        .await?;
        Ok(vec![true; keys.len()].into_iter().collect::<Vec<bool>>())
    }

    async fn set_and_delete_many(
        &self,
        keys_to_set: &[KVQPair<&Vec<u8>, &Vec<u8>>],
        keys_to_delete: &[Vec<u8>],
    ) -> Result<()> {
        let namespace_bytes = self.namespace_bytes.clone();

        self.with_pessimistic_txn(|mut txn| async move {
            let mut mutations = Vec::new();
            for item in keys_to_set {
                mutations.push(Mutation {
                    op: Op::Put.into(),
                    key: prefix_key(&namespace_bytes, item.key).into(),
                    value: item.value.to_vec(),
                    ..Default::default()
                });
            }

            for key in keys_to_delete {
                mutations.push(Mutation {
                    op: Op::Del.into(),
                    key: prefix_key(&namespace_bytes, key).into(),
                    value: vec![],
                    ..Default::default()
                });
            }

            if !mutations.is_empty() {
                txn.batch_mutate(mutations).await?;
            }
            Ok(txn)
        })
        .await
    }
}

impl KVQBinaryStore for TiKVStore {
    fn get_exact_if_exists(&self, key: &Vec<u8>) -> Result<Option<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { KVQBinaryStoreAsync::get_exact_if_exists(self, key).await })
        })
    }

    fn get_exact(&self, key: &Vec<u8>) -> Result<Vec<u8>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { KVQBinaryStoreAsync::get_exact(self, key).await })
        })
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> Result<Vec<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { KVQBinaryStoreAsync::get_many_exact(self, keys).await })
        })
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> Result<Option<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { KVQBinaryStoreAsync::get_leq(self, key, fuzzy_bytes).await })
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
            tokio::runtime::Handle::current()
                .block_on(async { KVQBinaryStoreAsync::get_leq_kv(self, key, fuzzy_bytes).await })
        })
    }

    fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> Result<Vec<Option<Vec<u8>>>> {
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
            tokio::runtime::Handle::current()
                .block_on(async { KVQBinaryStoreAsync::set(self, key, value).await })
        })
    }

    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { KVQBinaryStoreAsync::set_ref(self, key, value).await })
        })
    }

    fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { KVQBinaryStoreAsync::set_many_ref(self, items).await })
        })
    }

    fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { KVQBinaryStoreAsync::set_many_vec(self, items).await })
        })
    }

    fn delete(&self, key: &Vec<u8>) -> Result<bool> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { KVQBinaryStoreAsync::delete(self, key).await })
        })
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> Result<Vec<bool>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { KVQBinaryStoreAsync::delete_many(self, keys).await })
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
