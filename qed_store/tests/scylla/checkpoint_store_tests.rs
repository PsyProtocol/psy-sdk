use kvq::traits::{KVQBinaryStoreAsync,   KVQPair};
use qed_store::store::scylla::checkpoint_store::{
    chop_table_key, unchop_table_key, ScyllaCheckpointStore,
};
use std::sync::Arc;

mod common;
use common::*;

#[cfg(test)]
mod checkpoint_store_basic_tests {
    use super::*;

    #[tokio::test]
    async fn test_checkpoint_store_creation() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;

        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        // Verify store was created successfully
        assert!(!config.keyspace.is_empty());
        assert!(!config.table_name.is_empty());

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_chop_unchop_table_key() {
        let node_uuid = b"test_node_uuid_12345".to_vec();
        let checkpoint_id = 12345u64;

        // Create full key
        let full_key = unchop_table_key(&node_uuid, checkpoint_id);

        // Chop it back
        let (chopped_uuid, chopped_checkpoint) = chop_table_key(&full_key);

        assert_eq!(chopped_uuid, node_uuid);
        assert_eq!(chopped_checkpoint, checkpoint_id);
    }

    #[tokio::test]
    async fn test_checkpoint_store_basic_operations() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"test_node_001".to_vec();
        let checkpoint_id = 100u64;
        let value = b"test_value_for_checkpoint_100".to_vec();

        // Create full key with checkpoint
        let full_key = unchop_table_key(&node_uuid, checkpoint_id);

        // Set and get
        store.set(full_key.clone(), value.clone()).await?;
        let retrieved = store.get_exact_if_exists(&full_key).await?;
        assert_eq!(retrieved, Some(value.clone()));

        // Test get_exact
        let exact_result = store.get_exact(&full_key).await?;
        assert_eq!(exact_result, value);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_time_travel() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"time_travel_node".to_vec();
        let value_v1 = b"value_at_checkpoint_100".to_vec();
        let value_v2 = b"value_at_checkpoint_200".to_vec();
        let value_v3 = b"value_at_checkpoint_300".to_vec();

        // Insert multiple versions of the same node
        let key_v1 = unchop_table_key(&node_uuid, 100);
        let key_v2 = unchop_table_key(&node_uuid, 200);
        let key_v3 = unchop_table_key(&node_uuid, 300);

        store.set(key_v1, value_v1.clone()).await?;
        store.set(key_v2, value_v2.clone()).await?;
        store.set(key_v3, value_v3.clone()).await?;

        // Test time travel queries using get_leq

        // Query at checkpoint 150 should return value from checkpoint 100
        let query_key_150 = unchop_table_key(&node_uuid, 150);
        let result_150 = store.get_leq(&query_key_150, 8).await?;
        assert_eq!(result_150, Some(value_v1));

        // Query at checkpoint 250 should return value from checkpoint 200
        let query_key_250 = unchop_table_key(&node_uuid, 250);
        let result_250 = store.get_leq(&query_key_250, 8).await?;
        assert_eq!(result_250, Some(value_v2));

        // Query at checkpoint 350 should return value from checkpoint 300
        let query_key_350 = unchop_table_key(&node_uuid, 350);
        let result_350 = store.get_leq(&query_key_350, 8).await?;
        assert_eq!(result_350, Some(value_v3.clone()));

        // Query at checkpoint 50 (before any data) should return None
        let query_key_50 = unchop_table_key(&node_uuid, 50);
        let result_50 = store.get_leq(&query_key_50, 8).await?;
        assert_eq!(result_50, None);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_get_leq_kv() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"test_node_kv".to_vec();
        let checkpoint_id = 500u64;
        let value = b"test_value_kv".to_vec();

        let full_key = unchop_table_key(&node_uuid, checkpoint_id);
        store.set(full_key.clone(), value.clone()).await?;

        // Test get_leq_kv
        let query_key = unchop_table_key(&node_uuid, 600); // Query at later checkpoint
        let result_kv = store.get_leq_kv(&query_key, 8).await?;

        assert!(result_kv.is_some());
        let kv_pair = result_kv.unwrap();
        assert_eq!(kv_pair.key, full_key);
        assert_eq!(kv_pair.value, value);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_get_many_exact() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        // Create multiple checkpoint keys
        let keys = generate_checkpoint_keys(5, 100);
        let values = generate_test_values(5);

        // Set all key-value pairs
        for (key, value) in keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Get multiple keys
        let retrieved_values = store.get_many_exact(&keys).await?;
        assert_eq!(retrieved_values, values);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_batch_operations() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let keys = generate_checkpoint_keys(10, 200);
        let values = generate_test_values(10);
        let items_ref = create_kvq_pairs_ref(&keys, &values);

        // Test batch set
        store.set_many_ref(&items_ref).await?;

        // Verify all items were set
        for (key, value) in keys.iter().zip(values.iter()) {
            let retrieved = store.get_exact(key).await?;
            assert_eq!(retrieved, *value);
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_optimized_batch_queries() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        // Create exactly 15 keys to test go_sel_15 optimization
        let node_uuids: Vec<Vec<u8>> = (0..15)
            .map(|i| format!("batch_node_{:02}", i).into_bytes())
            .collect();
        let checkpoint_id = 300u64;
        let values = generate_test_values(15);

        // Set all keys
        for (node_uuid, value) in node_uuids.iter().zip(values.iter()) {
            let full_key = unchop_table_key(node_uuid, checkpoint_id);
            store.set(full_key, value.clone()).await?;
        }

        // Test optimized batch query
        let result = store.go_sel_15(&node_uuids, checkpoint_id).await?;
        assert_eq!(result.len(), 15);

        // All results should match our values
        for (result_opt, expected_value) in result.iter().zip(values.iter()) {
            assert_eq!(result_opt.as_ref(), Some(expected_value));
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_get_many_values() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        // Create 20 keys to test batch processing (will use both go_sel_15 and individual queries)
        let node_uuids: Vec<Vec<u8>> = (0..20)
            .map(|i| format!("batch_node_{:02}", i).into_bytes())
            .collect();
        let checkpoint_id = 400u64;
        let values = generate_test_values(20);

        // Set all keys
        for (node_uuid, value) in node_uuids.iter().zip(values.iter()) {
            let full_key = unchop_table_key(node_uuid, checkpoint_id);
            store.set(full_key, value.clone()).await?;
        }

        // Test get_many_values (should use optimized batching)
        let result = store.get_many_values(&node_uuids, checkpoint_id).await?;
        assert_eq!(result.len(), 20);

        // All results should match our values
        for (result_opt, expected_value) in result.iter().zip(values.iter()) {
            assert_eq!(result_opt.as_ref(), Some(expected_value));
        }

        config.cleanup().await?;
        Ok(())
    }
}
