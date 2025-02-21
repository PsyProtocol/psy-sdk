use futures::StreamExt;
use kvq::traits::{KVQBinaryStoreReaderAsync, KVQBinaryStoreWriterAsync, KVQBinaryStoreWriterImmutableAsync, KVQPair};
use scylla::prepared_statement::PreparedStatement;
use scylla::{Session, SessionBuilder};
use scylla::batch::{Batch, BatchType};
use anyhow::Result;
use async_trait::async_trait;

use serde::{Serialize, Deserialize};

pub struct KVQScyllaDBStore {
    session: Session,
    table_name: String,
    prepared_insert: PreparedStatement,
    prepared_select: PreparedStatement,
    prepared_delete: PreparedStatement,
}

impl KVQScyllaDBStore {
    pub async fn new(uri: &str, keyspace: &str, table_name: &str) -> Result<Self> {
        let session = SessionBuilder::new()
            .known_node(uri)
            .build()
            .await?;

        // Ensure the keyspace and table exist.
        session.query_unpaged(format!(
            "CREATE KEYSPACE IF NOT EXISTS {} WITH replication = {{ 'class': 'SimpleStrategy', 'replication_factor': '1' }}",
            keyspace
        ), &[]).await?;

        session.query_unpaged(format!(
            "CREATE TABLE IF NOT EXISTS {}.{} (key blob PRIMARY KEY, value blob)",
            keyspace, table_name
        ), &[]).await?;

        let table_name = format!("{}.{}", keyspace, table_name);

        // Prepare statements.
        let prepared_insert = session.prepare(format!(
            "INSERT INTO {} (key, value) VALUES (?, ?)",
            table_name
        )).await?;

        let prepared_select = session.prepare(format!(
            "SELECT value FROM {} WHERE key = ?",
            table_name
        )).await?;

        let prepared_delete = session.prepare(format!(
            "DELETE FROM {} WHERE key = ?",
            table_name
        )).await?;

        Ok(Self {
            session,
            table_name,
            prepared_insert,
            prepared_select,
            prepared_delete,
        })
    }
}

#[async_trait]
impl KVQBinaryStoreReaderAsync for KVQScyllaDBStore {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> Result<Option<Vec<u8>>> {
        let res = self.session.execute_unpaged(&self.prepared_select, (key,)).await?.into_rows_result()?;
        match res.maybe_first_row::<(Vec<u8>,)>()? {
            Some(x) => {
                Ok(Some(x.0))
            },
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
        let key_prefix = &key[..key.len().saturating_sub(fuzzy_bytes)];
        let query = format!(
            "SELECT value FROM {} WHERE token(key) <= token(?) LIMIT 1",
            self.table_name
        );
        let prepared = self.session.prepare(query).await?;


        let res = self.session.execute_unpaged(&prepared, (key_prefix,)).await?.into_rows_result()?;
        match res.maybe_first_row::<(Vec<u8>,)>()? {
            Some(x) => {
                Ok(Some(x.0))
            },
            None => Ok(None),
        }
    }

    async fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        let key_prefix = &key[..key.len().saturating_sub(fuzzy_bytes)];
        let query = format!(
            "SELECT key, value FROM {} WHERE token(key) <= token(?)",
            self.table_name
        );
        let prepared = self.session.prepare(query).await?;
        let mut rows_stream = self.session.execute_iter(prepared, (key_prefix,)).await?.rows_stream::<(Vec<u8>,Vec<u8>)>()?;

        let mut result = Vec::new();
        
    while let Some(next_row_res) = rows_stream.next().await {
        let (key, value) = next_row_res?;
        result.push(KVQPair { key, value });

    }

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
}
#[async_trait]
impl KVQBinaryStoreWriterAsync for KVQScyllaDBStore {
    async fn set(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.session.execute_unpaged(&self.prepared_insert, (key, value)).await?;
        Ok(())
    }

    async fn set_ref(&mut self, key: &Vec<u8>, value: &Vec<u8>) -> Result<()> {
        self.set(key.clone(), value.clone()).await
    }

    async fn set_many_ref<'a>(
        &mut self,
        items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>],
    ) -> Result<()> {
        //todo fix
        for item in items.iter() {
            self.set_ref(item.key, item.value).await?;
        }
        /* 
        let mut batch = Batch::new(BatchType::Logged);

        for item in items {
            batch.append_statement_with_values(&self.prepared_insert, (item.key, item.value))?;
        }

        self.session.batch(&batch, &[]).await?;
        Ok(())*/

        Ok(())
    }

    async fn set_many_vec(&mut self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> Result<()> {
        let items_ref: Vec<_> = items
            .iter()
            .map(|item| KVQPair {
                key: &item.key,
                value: &item.value,
            })
            .collect();
        self.set_many_ref(&items_ref).await
    }

    async fn delete(&mut self, key: &Vec<u8>) -> Result<bool> {
        self.session.execute_unpaged(&self.prepared_delete, (key,)).await?;
        Ok(true) // Assume success since ScyllaDB doesn't indicate if the key existed.
    }

    async fn delete_many(&mut self, keys: &[Vec<u8>]) -> Result<Vec<bool>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.delete(key).await?);
        }
        Ok(results)
    }

    async fn set_many_split_ref(&mut self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> Result<()> {
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



#[async_trait]
impl KVQBinaryStoreWriterImmutableAsync for KVQScyllaDBStore {
    async fn imm_set(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.session.execute_unpaged(&self.prepared_insert, (key, value)).await?;
        Ok(())
    }

    async fn imm_set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> Result<()> {
        self.imm_set(key.clone(), value.clone()).await
    }

    async fn imm_set_many_ref<'a>(
        &self,
        items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>],
    ) -> Result<()> {
        //todo fix
        for item in items.iter() {
            self.imm_set_ref(item.key, item.value).await?;
        }
        /* 
        let mut batch = Batch::new(BatchType::Logged);

        for item in items {
            batch.append_statement_with_values(&self.prepared_insert, (item.key, item.value))?;
        }

        self.session.batch(&batch, &[]).await?;
        Ok(())*/

        Ok(())
    }

    async fn imm_set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> Result<()> {
        let items_ref: Vec<_> = items
            .iter()
            .map(|item| KVQPair {
                key: &item.key,
                value: &item.value,
            })
            .collect();
        self.imm_set_many_ref(&items_ref).await
    }

    async fn imm_delete(&self, key: &Vec<u8>) -> Result<bool> {
        self.session.execute_unpaged(&self.prepared_delete, (key,)).await?;
        Ok(true) // Assume success since ScyllaDB doesn't indicate if the key existed.
    }

    async fn imm_delete_many(&self, keys: &[Vec<u8>]) -> Result<Vec<bool>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.imm_delete(key).await?);
        }
        Ok(results)
    }

    async fn imm_set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> Result<()> {
        if keys.len() != values.len() {
            return Err(anyhow::anyhow!("Keys and values must have the same length"));
        }
        let items: Vec<_> = keys
            .iter()
            .zip(values.iter())
            .map(|(key, value)| KVQPair { key, value })
            .collect();
        self.imm_set_many_ref(&items).await
    }
}