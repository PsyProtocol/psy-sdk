use super::config::ScyllaDBConfig;
use anyhow::Result;
use async_trait::async_trait;
use futures::{future::BoxFuture, stream::FuturesUnordered, StreamExt};
use kvq::traits::{KVQBinaryStoreAsync, KVQPair};
use scylla::batch::{Batch, BatchType};
use scylla::prepared_statement::PreparedStatement;
use scylla::{Session, SessionBuilder};
use std::sync::Arc;
use tokio::sync::Semaphore;

const BATCH_SIZE: usize = 15;
const MAX_CONCURRENT_REQUESTS: usize = 30;

#[derive(Debug)]
pub struct ScyllaClusteringStore {
    session: Arc<Session>,
    pub(crate) table_name: String,
    clustering_key_size: usize,
    prepared_insert: PreparedStatement,
    prepared_select: PreparedStatement,
    prepared_delete: PreparedStatement,
    prepared_select_partition: PreparedStatement,
    prepared_select_leq: PreparedStatement,
    prepared_select_15: PreparedStatement,
    prepared_delete_15: PreparedStatement,
    semaphore: Arc<Semaphore>,
}

impl ScyllaClusteringStore {
    pub async fn new(
        uri: &str,
        keyspace: &str,
        table_name: &str,
        clustering_key_size: usize,
    ) -> Result<Self> {
        Self::new_with_config(uri, keyspace, table_name, clustering_key_size, None).await
    }

    pub async fn new_with_config(
        uri: &str,
        keyspace: &str,
        table_name: &str,
        clustering_key_size: usize,
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

        Self::new_with_session(Arc::new(session), keyspace, table_name, clustering_key_size).await
    }

    pub async fn new_with_session(
        session: Arc<Session>,
        keyspace: &str,
        table_name: &str,
        clustering_key_size: usize,
    ) -> Result<Self> {
        session
            .query_unpaged(
                format!(
                    "CREATE TABLE IF NOT EXISTS {}.{} (
                        partition_key blob,
                        clustering_key blob,
                        value blob,
                        PRIMARY KEY (partition_key, clustering_key)
                    ) WITH CLUSTERING ORDER BY (clustering_key DESC)",
                    keyspace, table_name
                ),
                &[],
            )
            .await?;

        let table_name = format!("{}.{}", keyspace, table_name);

        let prepared_insert = session
            .prepare(format!(
                "INSERT INTO {} (partition_key, clustering_key, value) VALUES (?, ?, ?)",
                table_name
            ))
            .await?;

        let prepared_select = session
            .prepare(format!(
                "SELECT value FROM {} WHERE partition_key = ? AND clustering_key = ?",
                table_name
            ))
            .await?;

        let prepared_delete = session
            .prepare(format!(
                "DELETE FROM {} WHERE partition_key = ? AND clustering_key = ?",
                table_name
            ))
            .await?;

        let prepared_select_partition = session
            .prepare(format!(
                "SELECT clustering_key, value FROM {} WHERE partition_key = ?",
                table_name
            ))
            .await?;

        let prepared_select_leq = session
            .prepare(format!(
                "SELECT clustering_key, value FROM {} WHERE partition_key = ? AND clustering_key <= ? LIMIT 1",
                table_name
            ))
            .await?;

        let prepared_select_15 = prepared_select.clone();

        let prepared_delete_15 = prepared_delete.clone();

        Ok(Self {
            session: session.clone(),
            table_name,
            clustering_key_size,
            prepared_insert,
            prepared_select,
            prepared_delete,
            prepared_select_partition,
            prepared_select_leq,
            prepared_select_15,
            prepared_delete_15,
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
        })
    }

    fn split_key(&self, key: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        if key.len() < self.clustering_key_size {
            return Err(anyhow::anyhow!(
                "Key too short: expected at least {} bytes, got {}",
                self.clustering_key_size,
                key.len()
            ));
        }
        let partition_key_size = key.len() - self.clustering_key_size;
        let partition_key = key[..partition_key_size].to_vec();
        let clustering_key = key[partition_key_size..].to_vec();
        Ok((partition_key, clustering_key))
    }

    fn combine_key(&self, partition_key: &[u8], clustering_key: &[u8]) -> Vec<u8> {
        let mut key = Vec::with_capacity(partition_key.len() + clustering_key.len());
        key.extend_from_slice(partition_key);
        key.extend_from_slice(clustering_key);
        key
    }
}

#[async_trait]
impl KVQBinaryStoreAsync for ScyllaClusteringStore {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> Result<Option<Vec<u8>>> {
        let (partition_key, clustering_key) = self.split_key(key)?;
        let res = self
            .session
            .execute_unpaged(&self.prepared_select, (partition_key, clustering_key))
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
            let chunk_owned: Vec<Vec<u8>> = chunk.to_vec();

            futures.push(Box::pin(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let mut chunk_results = std::collections::HashMap::new();

                for key in &chunk_owned {
                    let value = self.get_exact(key).await?;
                    chunk_results.insert(key.clone(), value);
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

        let (partition_key, clustering_key) = self.split_key(key)?;

        if fuzzy_bytes == clustering_key.len() {
            let res = self
                .session
                .execute_unpaged(&self.prepared_select_leq, (partition_key, clustering_key))
                .await?
                .into_rows_result()?;

            match res.maybe_first_row::<(Vec<u8>, Vec<u8>)>()? {
                Some((_, value)) => Ok(Some(value)),
                None => Ok(None),
            }
        } else {
            if key.len() < fuzzy_bytes {
                return Ok(None);
            }

            let prefix_len = key.len() - fuzzy_bytes;
            let prefix = &key[..prefix_len];

            let mut start_key = prefix.to_vec();
            start_key.extend(vec![0x00; fuzzy_bytes]);

            let end_key = key;

            let query = format!(
                "SELECT partition_key, clustering_key, value FROM {} WHERE token(partition_key) >= token(?) AND token(partition_key) <= token(?) ALLOW FILTERING",
                self.table_name
            );
            let prepared = self.session.prepare(query).await?;

            let res = self
                .session
                .execute_unpaged(&prepared, (&start_key, end_key))
                .await?
                .into_rows_result()?;

            let mut candidates = Vec::new();

            if let Ok(rows) = res.rows() {
                for row in rows {
                    let (pk, ck, value): (Vec<u8>, Vec<u8>, Vec<u8>) = row?;
                    let row_key = self.combine_key(&pk, &ck);

                    if row_key.len() == key.len() && row_key.starts_with(prefix) && row_key <= *key {
                        candidates.push((row_key, value));
                    }
                }
            }

            if candidates.is_empty() {
                let query = format!(
                    "SELECT partition_key, clustering_key, value FROM {} ALLOW FILTERING",
                    self.table_name
                );
                let prepared = self.session.prepare(query).await?;

                let res = self
                    .session
                    .execute_unpaged(&prepared, ())
                    .await?
                    .into_rows_result()?;

                if let Ok(rows) = res.rows() {
                    for row in rows {
                        let (pk, ck, value): (Vec<u8>, Vec<u8>, Vec<u8>) = row?;
                        let row_key = self.combine_key(&pk, &ck);

                        if row_key.len() == key.len() && row_key.starts_with(prefix) && row_key <= *key {
                            candidates.push((row_key, value));
                        }
                    }
                }
            }

            Ok(candidates
                .into_iter()
                .max_by(|a, b| a.0.cmp(&b.0))
                .map(|(_, value)| value))
        }
    }

    async fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        if fuzzy_bytes == 0 {
            match self.get_exact_if_exists(key).await? {
                Some(value) => {
                    return Ok(vec![KVQPair {
                        key: key.clone(),
                        value,
                    }]);
                }
                None => {
                    return Ok(vec![]);
                }
            }
        }

        let (partition_key, clustering_key) = self.split_key(key)?;

        if fuzzy_bytes == clustering_key.len() {
            let res = self
                .session
                .execute_unpaged(&self.prepared_select_partition, (partition_key.clone(),))
                .await?
                .into_rows_result()?;

            let mut result = Vec::new();
            if let Ok(rows) = res.rows() {
                for row in rows {
                    let (ck, value): (Vec<u8>, Vec<u8>) = row?;
                    if ck <= clustering_key {
                        let full_key = self.combine_key(&partition_key, &ck);
                        result.push(KVQPair {
                            key: full_key,
                            value,
                        });
                    }
                }
            }

            result.sort_by(|a, b| {
                let (_, ck_a) = self.split_key(&a.key).unwrap();
                let (_, ck_b) = self.split_key(&b.key).unwrap();
                ck_a.cmp(&ck_b)
            });

            Ok(result)
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
                "SELECT partition_key, clustering_key, value FROM {} WHERE token(partition_key) >= token(?) AND token(partition_key) <= token(?) ALLOW FILTERING",
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
                    let (pk, ck, value): (Vec<u8>, Vec<u8>, Vec<u8>) = row?;
                    let row_key = self.combine_key(&pk, &ck);

                    if row_key.len() == key.len() && row_key.starts_with(prefix) && row_key <= *key {
                        results.push(KVQPair {
                            key: row_key,
                            value,
                        });
                    }
                }
            }

            if results.is_empty() {
                let query = format!(
                    "SELECT partition_key, clustering_key, value FROM {} ALLOW FILTERING",
                    self.table_name
                );
                let prepared = self.session.prepare(query).await?;

                let res = self
                    .session
                    .execute_unpaged(&prepared, ())
                    .await?
                    .into_rows_result()?;

                if let Ok(rows) = res.rows() {
                    for row in rows {
                        let (pk, ck, value): (Vec<u8>, Vec<u8>, Vec<u8>) = row?;
                        let row_key = self.combine_key(&pk, &ck);

                        if row_key.len() == key.len() && row_key.starts_with(prefix) && row_key <= *key {
                            results.push(KVQPair {
                                key: row_key,
                                value,
                            });
                        }
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
        let (partition_key, clustering_key) = self.split_key(&key)?;
        self.session
            .execute_unpaged(
                &self.prepared_insert,
                (partition_key, clustering_key, value),
            )
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
            let clustering_key_size = self.clustering_key_size;

            let mut batch = Batch::new(BatchType::Logged);
            batch.set_is_idempotent(true);

            let mut values = Vec::new();
            for item in chunk {
                if item.key.len() < clustering_key_size {
                    return Err(anyhow::anyhow!(
                        "Key too short: expected at least {} bytes, got {}",
                        clustering_key_size,
                        item.key.len()
                    ));
                }
                let partition_key_size = item.key.len() - clustering_key_size;
                let partition_key = item.key[..partition_key_size].to_vec();
                let clustering_key = item.key[partition_key_size..].to_vec();
                batch.append_statement(prepared_insert.clone());
                values.push((partition_key, clustering_key, item.value.clone()));
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
        let (partition_key, clustering_key) = self.split_key(key)?;
        self.session
            .execute_unpaged(&self.prepared_delete, (partition_key, clustering_key))
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
            let table_name = self.table_name.clone();
            let clustering_key_size = self.clustering_key_size;

                let prepared_delete = self.prepared_delete.clone();
                let mut batch = Batch::new(BatchType::Logged);
                batch.set_is_idempotent(true);

                let mut values = Vec::new();
                for key in chunk {
                    if key.len() < clustering_key_size {
                        return Err(anyhow::anyhow!(
                            "Key too short: expected at least {} bytes, got {}",
                            clustering_key_size,
                            key.len()
                        ));
                    }
                    let partition_key_size = key.len() - clustering_key_size;
                    let partition_key = key[..partition_key_size].to_vec();
                    let clustering_key = key[partition_key_size..].to_vec();
                    batch.append_statement(prepared_delete.clone());
                    values.push((partition_key, clustering_key));
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
}
