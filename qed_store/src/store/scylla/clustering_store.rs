use anyhow::Result;
use async_trait::async_trait;
use kvq::traits::{
    KVQBinaryStoreAsync,
    KVQPair,
};
use scylla::batch::{Batch, BatchType};
use scylla::prepared_statement::PreparedStatement;
use scylla::{Session, SessionBuilder};
use std::sync::Arc;
use super::config::ScyllaDBConfig;

/// A ScyllaDB store that uses clustering keys for efficient range queries
/// The key format should encode both partition_key and clustering_key
#[derive(Debug)]
pub struct ScyllaClusteringStore {
    session: Arc<Session>,
    pub(crate) table_name: String,
    partition_key_size: usize,
    prepared_insert: PreparedStatement,
    prepared_select: PreparedStatement,
    prepared_delete: PreparedStatement,
    prepared_select_partition: PreparedStatement,
    prepared_select_leq: PreparedStatement,
}

impl ScyllaClusteringStore {
    pub async fn new(
        uri: &str,
        keyspace: &str,
        table_name: &str,
        partition_key_size: usize,
    ) -> Result<Self> {
        Self::new_with_config(uri, keyspace, table_name, partition_key_size, None).await
    }

    pub async fn new_with_config(
        uri: &str,
        keyspace: &str,
        table_name: &str,
        partition_key_size: usize,
        config: Option<&ScyllaDBConfig>,
    ) -> Result<Self> {
        let session = SessionBuilder::new().known_node(uri).build().await?;

        // Create keyspace if using standalone
        let default_config = ScyllaDBConfig::default();
        let config = config.unwrap_or(&default_config);

        let replication_clause = if config.replication_class == "NetworkTopologyStrategy" {
            format!("{{'class': 'NetworkTopologyStrategy', 'datacenter1': {}}}", config.replication_factor)
        } else {
            format!("{{'class': '{}', 'replication_factor': {}}}", config.replication_class, config.replication_factor)
        };

        session.query_unpaged(format!(
            "CREATE KEYSPACE IF NOT EXISTS {} WITH replication = {}",
            keyspace, replication_clause
        ), &[]).await?;

        Self::new_with_session(Arc::new(session), keyspace, table_name, partition_key_size).await
    }

    pub async fn new_with_session(
        session: Arc<Session>,
        keyspace: &str,
        table_name: &str,
        partition_key_size: usize,
    ) -> Result<Self> {
        // Keyspace should already be created by ScyllaStore

        // Create table with partition key and clustering key
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

        // Prepare statements
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

        // Query all rows in a partition
        let prepared_select_partition = session
            .prepare(format!(
                "SELECT clustering_key, value FROM {} WHERE partition_key = ?",
                table_name
            ))
            .await?;

        // Query rows with clustering key less than or equal to a given value
        let prepared_select_leq = session
            .prepare(format!(
                "SELECT clustering_key, value FROM {} WHERE partition_key = ? AND clustering_key <= ? LIMIT 1",
                table_name
            ))
            .await?;

        Ok(Self {
            session: session.clone(),
            table_name,
            partition_key_size,
            prepared_insert,
            prepared_select,
            prepared_delete,
            prepared_select_partition,
            prepared_select_leq,
        })
    }

    fn split_key(&self, key: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        if key.len() < self.partition_key_size {
            return Err(anyhow::anyhow!(
                "Key too short: expected at least {} bytes, got {}",
                self.partition_key_size,
                key.len()
            ));
        }
        let partition_key = key[..self.partition_key_size].to_vec();
        let clustering_key = key[self.partition_key_size..].to_vec();
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
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            result.push(self.get_exact(key).await?);
        }
        Ok(result)
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> Result<Option<Vec<u8>>> {
        if fuzzy_bytes == 0 {
            return self.get_exact_if_exists(key).await;
        }

        let (partition_key, clustering_key) = self.split_key(key)?;

        // Use prepared statement for efficient lookup
        let res = self
            .session
            .execute_unpaged(&self.prepared_select_leq, (partition_key, clustering_key))
            .await?
            .into_rows_result()?;

        match res.maybe_first_row::<(Vec<u8>, Vec<u8>)>()? {
            Some((_, value)) => Ok(Some(value)),
            None => Ok(None),
        }
    }

    async fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        let (partition_key, clustering_key) = self.split_key(key)?;

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

        // Sort by clustering key
        result.sort_by(|a, b| {
            let (_, ck_a) = self.split_key(&a.key).unwrap();
            let (_, ck_b) = self.split_key(&b.key).unwrap();
            ck_a.cmp(&ck_b)
        });

        Ok(result)
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
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.get_leq(key, fuzzy_bytes).await?);
        }
        Ok(results)
    }

    async fn get_many_leq_kv(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> Result<Vec<Option<KVQPair<Vec<u8>, Vec<u8>>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.get_leq_kv(key, fuzzy_bytes).await?);
        }
        Ok(results)
    }

    // Write operations
    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let (partition_key, clustering_key) = self.split_key(&key)?;
        self.session
            .execute_unpaged(&self.prepared_insert, (partition_key, clustering_key, value))
            .await?;
        Ok(())
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> Result<()> {
        self.set(key.clone(), value.clone()).await
    }

    async fn set_many_ref<'a>(
        &self,
        items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>],
    ) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        // For small batches, use prepared batches for better performance
        if items.len() <= 16 {
            let mut batch = Batch::new(BatchType::Logged);
            batch.set_is_idempotent(true);

            let mut values = Vec::new();
            for item in items {
                let (partition_key, clustering_key) = self.split_key(item.key)?;
                batch.append_statement(self.prepared_insert.clone());
                values.push((partition_key, clustering_key, item.value.clone()));
            }

            self.session.batch(&batch, values).await?;
        } else {
            // For large batches, fall back to individual inserts to avoid timeouts
            for item in items.iter() {
                self.set_ref(item.key, item.value).await?;
            }
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
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.delete(key).await?);
        }
        Ok(results)
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

