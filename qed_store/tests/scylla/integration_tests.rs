use kvq::traits::{
    KVQBinaryStoreReaderAsync, KVQBinaryStoreWriterAsync, KVQBinaryStoreWriterImmutableAsync,
    KVQPair, KVQSerializable,
};
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_store::store::scylla::{
    checkpoint_store::ScyllaCheckpointStore,
    kvq_store::ScyllaKVQStore,
    merkle_store::ScyllaMerkleStore,
};
use qed_store::{
    models::kvq_merkle::key::KVQMerkleNodeKey,
    traits::merkle_store::{
        MerkleNodeStoreReaderImmutableAsync, MerkleNodeStoreWriterImmutableAsync,
    },
};
use scylla::{Session, SessionBuilder};
use std::sync::Arc;
use tokio;

// Test utilities and setup
pub struct TestConfig {
    pub keyspace: String,
    pub table_name: String,
    pub session: Arc<Session>,
}

impl TestConfig {
    pub async fn new() -> anyhow::Result<Self> {
        let uri = std::env::var("SCYLLA_URI").unwrap_or_else(|_| "127.0.0.1:9042".to_string());
        let keyspace = format!(
            "test_ks_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );
        let table_name = format!(
            "test_tbl_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );

        let session = Arc::new(SessionBuilder::new().known_node(&uri).build().await?);

        Ok(TestConfig {
            keyspace,
            table_name,
            session,
        })
    }

    pub async fn cleanup(&self) -> anyhow::Result<()> {
        // Drop keyspace after tests
        let drop_query = format!("DROP KEYSPACE IF EXISTS {}", self.keyspace);
        self.session.query_unpaged(drop_query, &[]).await?;
        Ok(())
    }
}

// Test data generators
pub fn generate_test_keys(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| format!("test_key_{:04}", i).into_bytes())
        .collect()
}

pub fn generate_test_values(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| format!("test_value_{:04}", i).into_bytes())
        .collect()
}

pub fn generate_test_kvpairs(count: usize) -> Vec<KVQPair<Vec<u8>, Vec<u8>>> {
    (0..count)
        .map(|i| KVQPair {
            key: format!("test_key_{:04}", i).into_bytes(),
            value: format!("test_value_{:04}", i).into_bytes(),
        })
        .collect()
}

pub fn generate_checkpoint_keys(count: usize, checkpoint_id: u64) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| {
            let mut key = format!("node_uuid_{:04}", i).into_bytes();
            key.extend_from_slice(&checkpoint_id.to_be_bytes());
            key
        })
        .collect()
}

pub fn generate_merkle_node_keys<const TABLE_TYPE: u16>(
    count: usize,
    checkpoint_id: u64,
) -> Vec<KVQMerkleNodeKey<TABLE_TYPE>> {
    (0..count)
        .map(|i| KVQMerkleNodeKey {
            tree_id: 1,
            primary_id: 0,
            secondary_id: 0,
            level: (i % 8) as u8,
            index: i as u64,
            checkpoint_id,
        })
        .collect()
}

pub fn generate_test_hashes(count: usize) -> Vec<QHashOut<GoldilocksField>> {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use qed_core::data::qhashout::QHashOut;

    (0..count)
        .map(|i| QHashOut::from_values(i as u64, (i * 2) as u64, (i * 3) as u64, (i * 4) as u64))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_setup_and_cleanup() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;

        // Test that we can create and connect to the session
        assert!(!config.keyspace.is_empty());
        assert!(!config.table_name.is_empty());

        // Test cleanup
        config.cleanup().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_data_generators() {
        let keys = generate_test_keys(5);
        let values = generate_test_values(5);
        let kvpairs = generate_test_kvpairs(5);

        assert_eq!(keys.len(), 5);
        assert_eq!(values.len(), 5);
        assert_eq!(kvpairs.len(), 5);

        // Check that generated data is different
        for i in 0..4 {
            assert_ne!(keys[i], keys[i + 1]);
            assert_ne!(values[i], values[i + 1]);
            assert_ne!(kvpairs[i].key, kvpairs[i + 1].key);
        }
    }
}
