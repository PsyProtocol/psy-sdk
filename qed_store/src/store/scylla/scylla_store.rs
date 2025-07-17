use anyhow::Result;
use scylla::{Session, SessionBuilder};
use std::sync::Arc;
use kvq::snapshot::SnapshotAsync;
use super::{
    clustering_store::ScyllaClusteringStore, config::ScyllaDBConfig, kvq_store::ScyllaKVQStore,
};

use qed_data::config::store_config::{
    CHECKPOINT_BLOCK_STATE_TABLE_TYPE, CHECKPOINT_HASH_HELPER_TABLE_TYPE,
    CHECKPOINT_LEAF_TABLE_TYPE, CHECKPOINT_SYNC_INFO_TABLE_TYPE, CHECKPOINT_TREE_TABLE_TYPE,
    CONTRACT_CODE_TABLE_TYPE, CONTRACT_FUNCTION_TREE_TABLE_TYPE, CONTRACT_LEAF_TABLE_TYPE,
    CONTRACT_TREE_TABLE_TYPE, DEPOSIT_TREE_TABLE_TYPE, USER_CONTRACT_STATE_TREE_TABLE_TYPE,
    USER_CONTRACT_TREE_TABLE_TYPE, USER_LEAF_TABLE_TYPE, USER_PUBLIC_KEY_HELPER_TABLE_TYPE,
    USER_REGISTRATION_TREE_TABLE_TYPE, USER_TREE_TABLE_TYPE, WITHDRAWAL_TREE_TABLE_TYPE,
};

use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair, ScyllaKey};

#[async_trait::async_trait]
pub trait ScyllaStoreInstance: KVQBinaryStoreAsync + Send + Sync {
    fn table_name(&self) -> &str;
}

#[async_trait::async_trait]
impl ScyllaStoreInstance for ScyllaKVQStore {
    fn table_name(&self) -> &str {
        &self.table_name
    }
}

#[async_trait::async_trait]
impl ScyllaStoreInstance for ScyllaClusteringStore {
    fn table_name(&self) -> &str {
        &self.table_name
    }
}

pub struct ScyllaStore {
    session: Arc<Session>,
    config: ScyllaDBConfig,
    stores: [Option<Arc<dyn ScyllaStoreInstance>>; 50],
}

unsafe impl Send for ScyllaStore {}
unsafe impl Sync for ScyllaStore {}

impl std::fmt::Debug for ScyllaStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScyllaStore")
            .field("session", &"<Session>")
            .field("config", &self.config)
            .field("stores", &"<stores>")
            .finish()
    }
}

impl ScyllaStore {
    pub async fn new(uri: &str, keyspace: &str) -> Result<Self> {
        Self::new_with_config(uri, keyspace, None).await
    }

    pub async fn new_with_config(
        uri: &str,
        keyspace: &str,
        config: Option<ScyllaDBConfig>,
    ) -> Result<Self> {
        let config = config.unwrap_or_default();

        let session = SessionBuilder::new().known_node(uri).build().await?;

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

        let mut store = Self {
            session: Arc::new(session),
            config,
            stores: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            ],
        };

        store.register_all_tables(keyspace).await?;

        Ok(store)
    }

    async fn register_all_tables(&mut self, keyspace: &str) -> Result<()> {
        self.register_clustering_table(CHECKPOINT_TREE_TABLE_TYPE, keyspace, "checkpoint_trees", 8)
            .await?;
        self.register_clustering_table(USER_TREE_TABLE_TYPE, keyspace, "user_trees", 8)
            .await?;
        self.register_clustering_table(CONTRACT_TREE_TABLE_TYPE, keyspace, "contract_trees", 8)
            .await?;
        self.register_clustering_table(
            CONTRACT_FUNCTION_TREE_TABLE_TYPE,
            keyspace,
            "contract_function_trees",
            8,
        )
        .await?;
        self.register_clustering_table(DEPOSIT_TREE_TABLE_TYPE, keyspace, "deposit_trees", 8)
            .await?;
        self.register_clustering_table(WITHDRAWAL_TREE_TABLE_TYPE, keyspace, "withdrawal_trees", 8)
            .await?;
        self.register_clustering_table(
            USER_REGISTRATION_TREE_TABLE_TYPE,
            keyspace,
            "user_registration_trees",
            8,
        )
        .await?;
        self.register_clustering_table(
            USER_CONTRACT_TREE_TABLE_TYPE,
            keyspace,
            "user_contract_trees",
            8,
        )
        .await?;
        self.register_clustering_table(
            USER_CONTRACT_STATE_TREE_TABLE_TYPE,
            keyspace,
            "user_contract_state_trees",
            8,
        )
        .await?;

        self.register_clustering_table(USER_LEAF_TABLE_TYPE, keyspace, "user_leaves", 4)
            .await?;

        self.register_clustering_table(
            CHECKPOINT_LEAF_TABLE_TYPE,
            keyspace,
            "checkpoint_leaves",
            8,
        )
        .await?;

        self.register_clustering_table(
            CHECKPOINT_BLOCK_STATE_TABLE_TYPE,
            keyspace,
            "checkpoint_block_states",
            8,
        )
        .await?;

        self.register_clustering_table(CONTRACT_LEAF_TABLE_TYPE, keyspace, "contract_leaves", 4)
            .await?;

        self.register_clustering_table(CONTRACT_CODE_TABLE_TYPE, keyspace, "contract_codes", 4)
            .await?;

        self.register_clustering_table(
            CHECKPOINT_SYNC_INFO_TABLE_TYPE,
            keyspace,
            "checkpoint_sync_info",
            8,
        )
        .await?;

        self.register_kvq_table(
            CHECKPOINT_HASH_HELPER_TABLE_TYPE,
            keyspace,
            "checkpoint_hash_helpers",
        )
        .await?;

        self.register_kvq_table(
            USER_PUBLIC_KEY_HELPER_TABLE_TYPE,
            keyspace,
            "user_public_key_helpers",
        )
        .await?;

        Ok(())
    }

    async fn register_kvq_table(
        &mut self,
        table_type: u16,
        keyspace: &str,
        table_name: &str,
    ) -> Result<()> {
        if table_type as usize >= 50 {
            return Err(anyhow::anyhow!(
                "Table type {} exceeds maximum of 49",
                table_type
            ));
        }

        let store =
            ScyllaKVQStore::new_with_session(self.session.clone(), keyspace, table_name).await?;

        self.stores[table_type as usize] = Some(Arc::new(store));
        Ok(())
    }

    async fn register_clustering_table(
        &mut self,
        table_type: u16,
        keyspace: &str,
        table_name: &str,
        clustering_key_size: usize,
    ) -> Result<()> {
        if table_type as usize >= 50 {
            return Err(anyhow::anyhow!(
                "Table type {} exceeds maximum of 49",
                table_type
            ));
        }

        let store = ScyllaClusteringStore::new_with_session(
            self.session.clone(),
            keyspace,
            table_name,
            clustering_key_size,
        )
        .await?;

        self.stores[table_type as usize] = Some(Arc::new(store));
        Ok(())
    }

    pub fn get_store(&self, table_type: u16) -> Result<Arc<dyn ScyllaStoreInstance>> {
        let index = table_type as usize;
        if index >= 50 {
            return Err(anyhow::anyhow!(
                "Table type {} exceeds maximum of 49",
                table_type
            ));
        }

        self.stores[index]
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No store registered for table type {}", table_type))
    }

    pub fn get_table_description(table_type: u16) -> &'static str {
        match table_type {
            CHECKPOINT_TREE_TABLE_TYPE => "Checkpoint Merkle Tree",
            USER_TREE_TABLE_TYPE => "User Merkle Tree",
            CONTRACT_TREE_TABLE_TYPE => "Contract Merkle Tree",
            CONTRACT_FUNCTION_TREE_TABLE_TYPE => "Contract Function Merkle Tree",
            DEPOSIT_TREE_TABLE_TYPE => "Deposit Merkle Tree",
            WITHDRAWAL_TREE_TABLE_TYPE => "Withdrawal Merkle Tree",
            USER_REGISTRATION_TREE_TABLE_TYPE => "User Registration Merkle Tree",
            USER_CONTRACT_TREE_TABLE_TYPE => "User Contract Merkle Tree",
            USER_CONTRACT_STATE_TREE_TABLE_TYPE => "User Contract State Merkle Tree",
            USER_LEAF_TABLE_TYPE => "User Leaf Data",
            CHECKPOINT_LEAF_TABLE_TYPE => "Checkpoint Leaf Data",
            CHECKPOINT_BLOCK_STATE_TABLE_TYPE => "Checkpoint Block State",
            CONTRACT_LEAF_TABLE_TYPE => "Contract Leaf Data",
            CONTRACT_CODE_TABLE_TYPE => "Contract Code Storage",
            CHECKPOINT_SYNC_INFO_TABLE_TYPE => "Checkpoint Sync Info",
            CHECKPOINT_HASH_HELPER_TABLE_TYPE => "Checkpoint Hash Helper",
            USER_PUBLIC_KEY_HELPER_TABLE_TYPE => "User Public Key Helper",
            _ => "Unknown Table Type",
        }
    }
}

#[async_trait::async_trait]
impl KVQBinaryStoreAsync for ScyllaStore {
    async fn get_exact_if_exists(&self, key: &Vec<u8>) -> Result<Option<Vec<u8>>> {
        if key.len() < 2 {
            return Err(anyhow::anyhow!("Key too short to extract table type"));
        }
        let table_type = u16::from_be_bytes([key[0], key[1]]);
        let store = self.get_store(table_type)?;
        store.get_exact_if_exists(key).await
    }

    async fn get_exact(&self, key: &Vec<u8>) -> Result<Vec<u8>> {
        <Self as KVQBinaryStoreAsync>::get_exact_if_exists(self, key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Key not found"))
    }

    async fn get_many_exact(&self, keys: &[Vec<u8>]) -> Result<Vec<Vec<u8>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(<Self as KVQBinaryStoreAsync>::get_exact(self, key).await?);
        }
        Ok(results)
    }

    async fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> Result<Option<Vec<u8>>> {
        if key.len() < 2 {
            return Err(anyhow::anyhow!("Key too short to extract table type"));
        }
        let table_type = u16::from_be_bytes([key[0], key[1]]);
        let store = self.get_store(table_type)?;
        store.get_leq(key, fuzzy_bytes).await
    }

    async fn get_fuzzy_range_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Vec<KVQPair<Vec<u8>, Vec<u8>>>> {
        if key.len() < 2 {
            return Err(anyhow::anyhow!("Key too short to extract table type"));
        }
        let table_type = u16::from_be_bytes([key[0], key[1]]);
        let store = self.get_store(table_type)?;
        store.get_fuzzy_range_leq_kv(key, fuzzy_bytes).await
    }

    async fn get_leq_kv(
        &self,
        key: &Vec<u8>,
        fuzzy_bytes: usize,
    ) -> Result<Option<KVQPair<Vec<u8>, Vec<u8>>>> {
        if key.len() < 2 {
            return Err(anyhow::anyhow!("Key too short to extract table type"));
        }
        let table_type = u16::from_be_bytes([key[0], key[1]]);
        let store = self.get_store(table_type)?;
        store.get_leq_kv(key, fuzzy_bytes).await
    }

    async fn get_many_leq(
        &self,
        keys: &[Vec<u8>],
        fuzzy_bytes: usize,
    ) -> Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(<Self as KVQBinaryStoreAsync>::get_leq(self, key, fuzzy_bytes).await?);
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
            results.push(<Self as KVQBinaryStoreAsync>::get_leq_kv(self, key, fuzzy_bytes).await?);
        }
        Ok(results)
    }

    async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        if key.len() < 2 {
            return Err(anyhow::anyhow!("Key too short to extract table type"));
        }
        let table_type = u16::from_be_bytes([key[0], key[1]]);
        let store = self.get_store(table_type)?;
        store.set(key, value).await
    }

    async fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> Result<()> {
        <Self as KVQBinaryStoreAsync>::set(self, key.clone(), value.clone()).await
    }

    async fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> Result<()> {
        for item in items {
            <Self as KVQBinaryStoreAsync>::set_ref(self, item.key, item.value).await?;
        }
        Ok(())
    }

    async fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> Result<()> {
        for item in items {
            <Self as KVQBinaryStoreAsync>::set(self, item.key, item.value).await?;
        }
        Ok(())
    }

    async fn delete(&self, key: &Vec<u8>) -> Result<bool> {
        if key.len() < 2 {
            return Err(anyhow::anyhow!("Key too short to extract table type"));
        }
        let table_type = u16::from_be_bytes([key[0], key[1]]);
        let store = self.get_store(table_type)?;
        store.delete(key).await
    }

    async fn delete_many(&self, keys: &[Vec<u8>]) -> Result<Vec<bool>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(<Self as KVQBinaryStoreAsync>::delete(self, key).await?);
        }
        Ok(results)
    }

    async fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> Result<()> {
        if keys.len() != values.len() {
            return Err(anyhow::anyhow!("Keys and values must have the same length"));
        }
        for (key, value) in keys.iter().zip(values.iter()) {
            <Self as KVQBinaryStoreAsync>::set_ref(self, key, value).await?;
        }
        Ok(())
    }
    async fn set_and_delete_many(
        &self,
        keys_to_set: &[KVQPair<&Vec<u8>, &Vec<u8>>],
        keys_to_delete: &[Vec<u8>]
    ) -> Result<()> {
        let mut keys = keys_to_set.iter().map(|kvq| kvq.key.clone()).collect::<Vec<_>>();
        let mut delete = keys_to_delete.to_vec();
        keys.append(&mut delete);
        let snapshot = self.create_snapshot(keys).await?;
        if let Err(err) = <Self as KVQBinaryStoreAsync>::set_many_ref(self, keys_to_set).await {
            self.restore_from_snapshot(snapshot).await?;
            return Err(err);
        }
        if let Err(err) = <Self as KVQBinaryStoreAsync>::delete_many(self, keys_to_delete).await {
            self.restore_from_snapshot(snapshot).await?;
            return Err(err);
        }
        Ok(())
    }
}

impl KVQBinaryStore for ScyllaStore {
    fn get_exact_if_exists(&self, key: &Vec<u8>) -> Result<Option<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                <Self as KVQBinaryStoreAsync>::get_exact_if_exists(self, key).await
            })
        })
    }

    fn get_exact(&self, key: &Vec<u8>) -> Result<Vec<u8>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { <Self as KVQBinaryStoreAsync>::get_exact(self, key).await })
        })
    }

    fn get_many_exact(&self, keys: &[Vec<u8>]) -> Result<Vec<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { <Self as KVQBinaryStoreAsync>::get_many_exact(self, keys).await })
        })
    }

    fn get_leq(&self, key: &Vec<u8>, fuzzy_bytes: usize) -> Result<Option<Vec<u8>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                <Self as KVQBinaryStoreAsync>::get_leq(self, key, fuzzy_bytes).await
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
                <Self as KVQBinaryStoreAsync>::get_fuzzy_range_leq_kv(self, key, fuzzy_bytes).await
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
                <Self as KVQBinaryStoreAsync>::get_leq_kv(self, key, fuzzy_bytes).await
            })
        })
    }

    fn get_many_leq(&self, keys: &[Vec<u8>], fuzzy_bytes: usize) -> Result<Vec<Option<Vec<u8>>>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                <Self as KVQBinaryStoreAsync>::get_many_leq(self, keys, fuzzy_bytes).await
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
                <Self as KVQBinaryStoreAsync>::get_many_leq_kv(self, keys, fuzzy_bytes).await
            })
        })
    }

    fn set(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { <Self as KVQBinaryStoreAsync>::set(self, key, value).await })
        })
    }

    fn set_ref(&self, key: &Vec<u8>, value: &Vec<u8>) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { <Self as KVQBinaryStoreAsync>::set_ref(self, key, value).await })
        })
    }

    fn set_many_ref<'a>(&self, items: &[KVQPair<&'a Vec<u8>, &'a Vec<u8>>]) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { <Self as KVQBinaryStoreAsync>::set_many_ref(self, items).await })
        })
    }

    fn set_many_vec(&self, items: Vec<KVQPair<Vec<u8>, Vec<u8>>>) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { <Self as KVQBinaryStoreAsync>::set_many_vec(self, items).await })
        })
    }

    fn delete(&self, key: &Vec<u8>) -> Result<bool> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { <Self as KVQBinaryStoreAsync>::delete(self, key).await })
        })
    }

    fn delete_many(&self, keys: &[Vec<u8>]) -> Result<Vec<bool>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { <Self as KVQBinaryStoreAsync>::delete_many(self, keys).await })
        })
    }

    fn set_many_split_ref(&self, keys: &[Vec<u8>], values: &[Vec<u8>]) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                <Self as KVQBinaryStoreAsync>::set_many_split_ref(self, keys, values).await
            })
        })
    }

    fn set_and_delete_many(
        &self,
        keys_to_set: &[KVQPair<&Vec<u8>, &Vec<u8>>],
        keys_to_delete: &[Vec<u8>]
    ) -> Result<()> {
        let mut keys = keys_to_set.iter().map(|kvq| kvq.key.clone()).collect::<Vec<_>>();
        let mut delete = keys_to_delete.to_vec();
        keys.append(&mut delete);
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                <Self as KVQBinaryStoreAsync>::set_and_delete_many(self, keys_to_set, keys_to_delete).await
            })
        })
    }
}
