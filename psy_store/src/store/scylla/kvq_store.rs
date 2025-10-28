use anyhow::Result;
use async_trait::async_trait;
use futures::{future::BoxFuture, stream::FuturesUnordered, StreamExt};
use kvq::traits::{KVQBinaryStoreAsync, KVQPair};
use scylla::batch::{Batch, BatchType};
use scylla::prepared_statement::PreparedStatement;
use scylla::{Session, SessionBuilder};
use std::sync::Arc;
use tokio::sync::Semaphore;

use super::config::ScyllaDBConfig;

const BATCH_SIZE: usize = 15;
const MAX_CONCURRENT_REQUESTS: usize = 30;

#[derive(Debug)]
pub struct ScyllaKVQStore {
    session: Arc<Session>,
    pub(crate) table_name: String,
    prepared_insert: PreparedStatement,
    prepared_select: PreparedStatement,
    prepared_delete: PreparedStatement,
    prepared_select_15: PreparedStatement,
    prepared_delete_15: PreparedStatement,
    semaphore: Arc<Semaphore>,
}

impl ScyllaKVQStore {
    pub async fn new(uri: &str, keyspace: &str, table_name: &str) -> Result<Self> {
        Self::new_with_config(uri, keyspace, table_name, None).await
    }

    pub async fn new_with_config(
        uri: &str,
        keyspace: &str,
        table_name: &str,
        config: Option<&ScyllaDBConfig>,
    ) -> Result<Self> {
        let session = SessionBuilder::new().known_node(uri).build().await?;

        let default_config = ScyllaDBConfig::default();
        let config = config.unwrap_or(&default_config);

        let replication_clause = if config.replication_class == "NetworkTopologyStrategy" {
            format!(
                "{{'class': 'NetworkTopologyStrategy', 'datacenter1': {}}}",
                config.replication_factor
            )
        } else {
            format!(
                "{{'class': '{}', 'replication_factor': {}}}",
                config.replication_class, config.replication_factor
            )
        };

        session
            .query_unpaged(
                format!(
                    "CREATE KEYSPACE IF NOT EXISTS {} WITH replication = {}",
                    keyspace, replication_clause
                ),
                &[],
            )
            .await?;

        Self::new_with_session(Arc::new(session), keyspace, table_name).await
    }

    pub async fn new_with_session(
        session: Arc<Session>,
        keyspace: &str,
        table_name: &str,
    ) -> Result<Self> {
        session
            .query_unpaged(
                format!(
                    "CREATE TABLE IF NOT EXISTS {}.{} (key blob PRIMARY KEY, value blob)",
                    keyspace, table_name
                ),
                &[],
            )
            .await?;

        let table_name = format!("{}.{}", keyspace, table_name);

        let prepared_insert = session
            .prepare(format!(
                "INSERT INTO {} (key, value) VALUES (?, ?)",
                table_name
            ))
            .await?;

        let prepared_select = session
            .prepare(format!("SELECT value FROM {} WHERE key = ?", table_name))
            .await?;

        let prepared_delete = session
            .prepare(format!("DELETE FROM {} WHERE key = ?", table_name))
            .await?;

        let placeholders: Vec<&str> = vec!["?"; BATCH_SIZE];
        let prepared_select_15 = session
            .prepare(format!(
                "SELECT key, value FROM {} WHERE key IN ({})",
                table_name,
                placeholders.join(",")
            ))
            .await?;

        let prepared_delete_15 = session
            .prepare(format!(
                "DELETE FROM {} WHERE key IN ({})",
                table_name,
                placeholders.join(",")
            ))
            .await?;

        Ok(Self {
            session: session.clone(),
            table_name,
            prepared_insert,
            prepared_select,
            prepared_delete,
            prepared_select_15,
            prepared_delete_15,
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
        })
    }
}

#[async_trait]
impl KVQBinaryStoreAsync for ScyllaKVQStore {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> Result<Option<Vec<u8>>> {
        let res = self
            .session
            .execute_unpaged(&self.prepared_select, (key,))
            .await?
            .into_rows_result()?;
        match res.maybe_first_row::<(Vec<u8>,)>()? {
            Some(x) => Ok(Some(x.0)),
            None => Ok(None),
        }
    }

    async fn get_exact(&self, key: &Vec<u8>) -> Result<Vec<u8>> {
        self.get_exact_if_exists(key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Key not found"))
    }

    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> Result<Vec<Vec<u8>>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_results = std::collections::HashMap::new();
        let mut futures: FuturesUnordered<
            BoxFuture<'_, Result<std::collections::HashMap<Vec<u8>, Vec<u8>>>>,
        > = FuturesUnordered::new();

        for chunk in keys.chunks(BATCH_SIZE) {
            let semaphore = self.semaphore.clone();
            let session = self.session.clone();
            let table_name = self.table_name.clone();
            let prepared = if chunk.len() == BATCH_SIZE {
                self.prepared_select_15.clone()
            } else {
                let placeholders: Vec<&str> = vec!["?"; chunk.len()];
                session
                    .prepare(format!(
                        "SELECT key, value FROM {} WHERE key IN ({})",
                        table_name,
                        placeholders.join(",")
                    ))
                    .await?
            };

            let mut values: Vec<&[u8]> = chunk.iter().map(|k| k.as_slice()).collect();
            if chunk.len() < BATCH_SIZE {
                values.resize(chunk.len(), &[]);
            } else {
                values.resize(BATCH_SIZE, &[]);
            }

            let chunk_owned: Vec<Vec<u8>> = chunk.to_vec();

            futures.push(Box::pin(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let res = session
                    .execute_unpaged(&prepared, values)
                    .await?
                    .into_rows_result()?;

                let mut chunk_results = std::collections::HashMap::new();
                if let Ok(rows) = res.rows() {
                    for row in rows {
                        let (key, value): (Vec<u8>, Vec<u8>) = row?;
                        chunk_results.insert(key, value);
                    }
                }

                for key in chunk_owned {
                    if !chunk_results.contains_key(&key) {
                        return Err(anyhow::anyhow!("Key not found"));
                    }
                }

                Ok::<_, anyhow::Error>(chunk_results)
            }));
        }

        while let Some(chunk_results) = futures.next().await {
            let chunk_results = chunk_results?;
            all_results.extend(chunk_results);
        }

        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            match all_results.remove(key) {
                Some(value) => results.push(value),
                None => return Err(anyhow::anyhow!("Key not found")),
            }
        }

        Ok(results)
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> Result<Option<Vec<u8>>> {
        if fuzzy_bytes == 0 {
            return self.get_exact_if_exists(key).await;
        }

        if key.len() < fuzzy_bytes {
            return Ok(None);
        }

        let prefix_len = key.len() - fuzzy_bytes;
        let prefix = &key[..prefix_len];

        let mut start_key = prefix.to_vec();
        start_key.extend(vec![0x00; fuzzy_bytes]);

        let end_key = key;

        let query = format!(
            "SELECT key, value FROM {} WHERE key >= ? AND key <= ? ALLOW FILTERING",
            self.table_name
        );
        let prepared = self.session.prepare(query).await?;

        let res = self
            .session
            .execute_unpaged(&prepared, (&start_key, end_key))
            .await?
            .into_rows_result()?;

        let mut best_match: Option<(Vec<u8>, Vec<u8>)> = None;

        if let Ok(rows) = res.rows() {
            for row in rows {
                let (row_key, row_value): (Vec<u8>, Vec<u8>) = row?;

                if row_key.len() == key.len() && row_key.starts_with(prefix) {
                    match &best_match {
                        None => best_match = Some((row_key, row_value)),
                        Some((best_key, _)) => {
                            if row_key > *best_key {
                                best_match = Some((row_key, row_value));
                            }
                        }
                    }
                }
            }
        }

        Ok(best_match.map(|(_, value)| value))
    }

    async fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        if fuzzy_bytes == 0 {
            match self.get_exact_if_exists(key).await? {
                Some(value) => Ok(vec![KVQPair {
                    key: key.clone(),
                    value,
                }]),
                None => Ok(vec![]),
            }
        } else {
            if key.len() < fuzzy_bytes {
                return Ok(vec![]);
            }

            let prefix_len = key.len() - fuzzy_bytes;
            let prefix = &key[..prefix_len];

            let mut start_key = prefix.to_vec();
            start_key.extend(vec![0x00; fuzzy_bytes]);

            let end_key = key;

            let query = format!(
                "SELECT key, value FROM {} WHERE key >= ? AND key <= ? ALLOW FILTERING",
                self.table_name
            );
            let prepared = self.session.prepare(query).await?;

            let res = self
                .session
                .execute_unpaged(&prepared, (&start_key, end_key))
                .await?
                .into_rows_result()?;

            let mut results = Vec::new();

            if let Ok(rows) = res.rows() {
                for row in rows {
                    let (row_key, row_value): (Vec<u8>, Vec<u8>) = row?;

                    if row_key.len() == key.len() && row_key.starts_with(prefix) {
                        results.push(KVQPair {
                            key: row_key,
                            value: row_value,
                        });
                    }
                }
            }

            results.sort_by(|a, b| a.key.cmp(&b.key));

            Ok(results)
        }
    }

    async fn get_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        let result = self.get_fuzzy_range_leq_kv(key, fuzzy_bytes).await?;
        Ok(result.last().cloned())
    }

    async fn get_many_leq(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> Result<Vec<Option<Vec<u8>>>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut futures: FuturesUnordered<BoxFuture<'_, Result<(usize, Vec<Option<Vec<u8>>>)>>> =
            FuturesUnordered::new();

        for (chunk_idx, chunk) in keys.chunks(BATCH_SIZE).enumerate() {
            let semaphore = self.semaphore.clone();
            let chunk_owned: Vec<Vec<u8>> = chunk.to_vec();

            futures.push(Box::pin(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let mut chunk_results = Vec::with_capacity(chunk_owned.len());

                for key in &chunk_owned {
                    chunk_results.push(self.get_leq(key, fuzzy_bytes).await?);
                }

                Ok::<_, anyhow::Error>((chunk_idx, chunk_results))
            }));
        }

        let mut indexed_results = Vec::new();
        while let Some(result) = futures.next().await {
            indexed_results.push(result?);
        }

        indexed_results.sort_by_key(|(idx, _)| *idx);

        let mut all_results = Vec::with_capacity(keys.len());
        for (_, chunk_results) in indexed_results {
            all_results.extend(chunk_results);
        }

        Ok(all_results)
    }

    async fn get_many_leq_kv(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut futures: FuturesUnordered<
            BoxFuture<'_, Result<(usize, Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>)>>,
        > = FuturesUnordered::new();

        for (chunk_idx, chunk) in keys.chunks(BATCH_SIZE).enumerate() {
            let semaphore = self.semaphore.clone();
            let chunk_owned: Vec<Vec<u8>> = chunk.to_vec();

            futures.push(Box::pin(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let mut chunk_results = Vec::with_capacity(chunk_owned.len());

                for key in &chunk_owned {
                    chunk_results.push(self.get_leq_kv(key, fuzzy_bytes).await?);
                }

                Ok::<_, anyhow::Error>((chunk_idx, chunk_results))
            }));
        }

        let mut indexed_results = Vec::new();
        while let Some(result) = futures.next().await {
            indexed_results.push(result?);
        }

        indexed_results.sort_by_key(|(idx, _)| *idx);

        let mut all_results = Vec::with_capacity(keys.len());
        for (_, chunk_results) in indexed_results {
            all_results.extend(chunk_results);
        }

        Ok(all_results)
    }

    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.session
            .execute_unpaged(&self.prepared_insert, (key, value))
            .await?;
        Ok(())
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> Result<()> {
        self.set(key.clone(), value.clone()).await
    }

    async fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let mut futures: FuturesUnordered<BoxFuture<'_, Result<()>>> = FuturesUnordered::new();

        for chunk in items.chunks(BATCH_SIZE) {
            let semaphore = self.semaphore.clone();
            let session = self.session.clone();
            let prepared_insert = self.prepared_insert.clone();

            let mut batch = Batch::new(BatchType::Logged);
            batch.set_is_idempotent(true);

            let mut values = Vec::new();
            for item in chunk {
                batch.append_statement(prepared_insert.clone());
                values.push((item.key.clone(), item.value.clone()));
            }

            futures.push(Box::pin(async move {
                let _permit = semaphore.acquire().await.unwrap();
                session.batch(&batch, values).await?;
                Ok::<_, anyhow::Error>(())
            }));
        }

        while let Some(result) = futures.next().await {
            result?;
        }

        Ok(())
    }

    async fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> Result<()> {
        let items_ref: Vec<_> = items
            .iter()
            .map(|item| KVQPair {
                key: &item.key,
                value: &item.value,
            })
            .collect();
        self.set_many_ref(&items_ref).await
    }

    async fn delete(&self, key: &Vec<u8>) -> Result<bool> {
        self.session
            .execute_unpaged(&self.prepared_delete, (key,))
            .await?;
        Ok(true)
    }

    async fn delete_many(&self, keys: &[Vec<u8>]) -> Result<Vec<bool>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut futures: FuturesUnordered<BoxFuture<'_, Result<()>>> = FuturesUnordered::new();

        for chunk in keys.chunks(BATCH_SIZE) {
            let semaphore = self.semaphore.clone();
            let session = self.session.clone();
            let prepared_delete = self.prepared_delete.clone();

            if chunk.len() == BATCH_SIZE {
                let values: Vec<&[u8]> = chunk.iter().map(|k| k.as_slice()).collect();
                let prepared = self.prepared_delete_15.clone();

                futures.push(Box::pin(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    session.execute_unpaged(&prepared, values).await?;
                    Ok::<_, anyhow::Error>(())
                }));
            } else {
                let mut batch = Batch::new(BatchType::Logged);
                batch.set_is_idempotent(true);

                let mut values = Vec::new();
                for key in chunk {
                    batch.append_statement(prepared_delete.clone());
                    values.push((key.clone(),));
                }

                futures.push(Box::pin(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    session.batch(&batch, values).await?;
                    Ok::<_, anyhow::Error>(())
                }));
            }
        }

        while let Some(result) = futures.next().await {
            result?;
        }

        Ok(vec![true; keys.len()])
    }

    async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> Result<()> {
        if keys.len() != values.len() {
            return Err(anyhow::anyhow!("Keys and values must have the same length"));
        }
        let items: Vec<_> = keys
            .iter()
            .zip(values.iter())
            .map(|(key, value)| KVQPair { key, value })
            .collect();
        self.set_many_ref(&items).await
    }

    async fn set_and_delete_many(
        &self,
        keys_to_set: &[KVQPair<&Vec<u8>, &Vec<u8>>],
        keys_to_delete: &[Vec<u8>],
    ) -> Result<()> {
        self.set_many_ref(keys_to_set).await?;
        self.delete_many(keys_to_delete).await?;
        Ok(())
    }
}
