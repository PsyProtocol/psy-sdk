use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use kvq::traits::{
    KVQBinaryStoreAsync,
    KVQPair,
};
use scylla::batch::{Batch, BatchType};
use scylla::prepared_statement::PreparedStatement;
use scylla::{Session, SessionBuilder};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use super::config::ScyllaDBConfig;

#[derive(Debug)]
pub struct ScyllaKVQStore {
    session: Arc<Session>,
    pub(crate) table_name: String,
    prepared_insert: PreparedStatement,
    prepared_select: PreparedStatement,
    prepared_delete: PreparedStatement,
}

impl ScyllaKVQStore {
    pub async fn new(uri: &str, keyspace: &str, table_name: &str) -> Result<Self> {
        Self::new_with_config(uri, keyspace, table_name, None).await
    }

    pub async fn new_with_config(
        uri: &str, 
        keyspace: &str, 
        table_name: &str, 
        config: Option<&ScyllaDBConfig>
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
        
        Self::new_with_session(Arc::new(session), keyspace, table_name).await
    }

    pub async fn new_with_session(
        session: Arc<Session>,
        keyspace: &str,
        table_name: &str,
    ) -> Result<Self> {
        // Keyspace should already be created by ScyllaStore

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

        // Prepare statements.
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

        Ok(Self {
            session: session.clone(),
            table_name,
            prepared_insert,
            prepared_select,
            prepared_delete,
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
        // TODO: query all at once
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            result.push(self.get_exact(key).await?);
        }
        Ok(result)
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> Result<Option<Vec<u8>>> {
        // Special case for checkpoint block state table
        // Handle both cases:
        // 1. When called through composite adapter: table type prefix is stripped, fuzzy_bytes = 6
        // 2. When called directly: table type prefix is present, fuzzy_bytes = 8
        
        let is_checkpoint_table = self.table_name.ends_with("checkpoint_block_states");
        
        
        if is_checkpoint_table && ((key.len() >= 8 && fuzzy_bytes == 6) || (key.len() >= 10 && fuzzy_bytes == 8)) {
            // Extract the requested checkpoint ID from the key
            let requested_id = if key.len() >= 10 && fuzzy_bytes == 8 {
                // Direct call: table type prefix is present
                u64::from_be_bytes([
                    key[2], key[3], key[4], key[5],
                    key[6], key[7], key[8], key[9],
                ])
            } else {
                // Through composite adapter: table type prefix is stripped
                u64::from_be_bytes([
                    key[0], key[1], key[2], key[3],
                    key[4], key[5], key[6], key[7],
                ])
            };
            
            // Just scan all checkpoints and find the maximum that's <= requested
            let query = format!("SELECT key, value FROM {} ALLOW FILTERING", self.table_name);
            let prepared = self.session.prepare(query).await?;
            
            let res = self
                .session
                .execute_unpaged(&prepared, ())
                .await?
                .into_rows_result()?;
            
            let mut best_match: Option<(Vec<u8>, Vec<u8>)> = None;
            
            if let Ok(rows) = res.rows() {
                let mut row_count = 0;
                for row in rows {
                    row_count += 1;
                    let (row_key, row_value): (Vec<u8>, Vec<u8>) = row?;
                    
                    // Handle both cases: with or without table type prefix
                    let row_id = if row_key.len() >= 10 {
                        // Key includes table type prefix
                        u64::from_be_bytes([
                            row_key[2], row_key[3], row_key[4], row_key[5],
                            row_key[6], row_key[7], row_key[8], row_key[9],
                        ])
                    } else if row_key.len() >= 8 {
                        // Key without table type prefix
                        u64::from_be_bytes([
                            row_key[0], row_key[1], row_key[2], row_key[3],
                            row_key[4], row_key[5], row_key[6], row_key[7],
                        ])
                    } else {
                        continue; // Skip invalid keys
                    };
                        
                    // Only consider rows where checkpoint_id <= requested_id
                    if row_id <= requested_id {
                        match &best_match {
                            None => best_match = Some((row_key, row_value)),
                            Some((best_key, _)) => {
                                // Compare checkpoint IDs
                                let best_id = if best_key.len() >= 10 {
                                    u64::from_be_bytes([
                                        best_key[2], best_key[3], best_key[4], best_key[5],
                                        best_key[6], best_key[7], best_key[8], best_key[9],
                                    ])
                                } else if best_key.len() >= 8 {
                                    u64::from_be_bytes([
                                        best_key[0], best_key[1], best_key[2], best_key[3],
                                        best_key[4], best_key[5], best_key[6], best_key[7],
                                    ])
                                } else {
                                    0 // Should not happen
                                };
                                if row_id > best_id {
                                    best_match = Some((row_key, row_value));
                                }
                            }
                        }
                    }
                }
            }
            
            return Ok(best_match.map(|(_, value)| value));
        }
        
        // For other cases, try exact match but return None if not found
        // TODO: Implement proper fuzzy matching for other table types
        match self.get_exact_if_exists(key).await? {
            Some(value) => Ok(Some(value)),
            None => Ok(None),
        }
    }

    async fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        // For now, we'll do a simple scan of all keys and filter
        // This is not efficient but works for small datasets
        let query = format!(
            "SELECT key, value FROM {}",
            self.table_name
        );
        let prepared = self.session.prepare(query).await?;
        let mut rows_stream = self
            .session
            .execute_iter(prepared, ())
            .await?
            .rows_stream::<(Vec<u8>, Vec<u8>)>()?;

        let mut result = Vec::new();
        let key_prefix = &key[..key.len().saturating_sub(fuzzy_bytes)];

        while let Some(next_row_res) = rows_stream.next().await {
            let (k, v) = next_row_res?;
            // Check if key matches prefix and is <= target key
            if k.len() >= key_prefix.len() && &k[..key_prefix.len()] == key_prefix && k <= *key {
                result.push(KVQPair { key: k, value: v });
            }
        }

        // Sort by key to ensure proper ordering
        result.sort_by(|a, b| a.key.cmp(&b.key));
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
        self.session
            .execute_unpaged(&self.prepared_insert, (key, value))
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
                batch.append_statement(self.prepared_insert.clone());
                values.push((item.key.clone(), item.value.clone()));
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
        self.session
            .execute_unpaged(&self.prepared_delete, (key,))
            .await?;
        Ok(true) // Assume success since ScyllaDB doesn't indicate if the key existed.
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

