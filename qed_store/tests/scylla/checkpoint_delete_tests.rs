use kvq::traits::KVQBinaryStoreAsync;
use qed_store::store::scylla::checkpoint_store::{
    chop_table_key, unchop_table_key, ScyllaCheckpointStore,
};
use std::sync::Arc;

mod common;
use common::*;

#[cfg(test)]
mod checkpoint_delete_tests {
    use super::*;

    #[tokio::test]
    async fn test_checkpoint_store_delete_single() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"delete_test_node".to_vec();
        let checkpoint_id = 100u64;
        let value = b"value_to_delete".to_vec();

        let full_key = unchop_table_key(&node_uuid, checkpoint_id);

        // Set the value
        store.set(full_key.clone(), value.clone()).await?;

        // Verify it exists
        let exists_before = store.get_exact_if_exists(&full_key).await?;
        assert_eq!(exists_before, Some(value));

        // Delete it
        let delete_result = store.delete(&full_key).await?;
        assert!(delete_result); // Should return true if existed

        // Verify it's gone
        let exists_after = store.get_exact_if_exists(&full_key).await?;
        assert_eq!(exists_after, None);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_delete_nonexistent() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"nonexistent_node".to_vec();
        let checkpoint_id = 999u64;
        let full_key = unchop_table_key(&node_uuid, checkpoint_id);

        // Try to delete non-existent key
        let delete_result = store.delete(&full_key).await?;
        assert!(!delete_result); // Should return false if didn't exist

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_delete_many() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        // Create multiple keys with different checkpoints
        let node_uuids = vec![
            b"delete_many_node_1".to_vec(),
            b"delete_many_node_2".to_vec(),
            b"delete_many_node_3".to_vec(),
        ];
        let checkpoint_ids = vec![100u64, 200u64, 300u64];
        let values = generate_test_values(3);

        let keys: Vec<Vec<u8>> = node_uuids
            .iter()
            .zip(checkpoint_ids.iter())
            .map(|(uuid, &checkpoint)| unchop_table_key(uuid, checkpoint))
            .collect();

        // Set all values
        for (key, value) in keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Verify all exist
        for (key, value) in keys.iter().zip(values.iter()) {
            let exists = store.get_exact_if_exists(key).await?;
            assert_eq!(exists, Some(value.clone()));
        }

        // Delete all keys
        let delete_results = store.delete_many(&keys).await?;
        assert_eq!(delete_results.len(), 3);
        assert!(delete_results.iter().all(|&result| result)); // All should return true

        // Verify all are gone
        for key in &keys {
            let exists = store.get_exact_if_exists(key).await?;
            assert_eq!(exists, None);
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_delete_many_mixed() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        // Create some keys that exist and some that don't
        let existing_node_uuids = vec![b"existing_node_1".to_vec(), b"existing_node_2".to_vec()];
        let nonexistent_node_uuids = vec![b"nonexist_node_1".to_vec(), b"nonexist_node_2".to_vec()];

        let existing_keys: Vec<Vec<u8>> = existing_node_uuids
            .iter()
            .map(|uuid| unchop_table_key(uuid, 100))
            .collect();
        let nonexistent_keys: Vec<Vec<u8>> = nonexistent_node_uuids
            .iter()
            .map(|uuid| unchop_table_key(uuid, 100))
            .collect();
        let values = generate_test_values(2);

        // Set only the existing keys
        for (key, value) in existing_keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Mix existing and non-existing keys
        let mut all_keys = existing_keys.clone();
        all_keys.extend(nonexistent_keys);

        // Delete all keys (mix of existing and non-existing)
        let delete_results = store.delete_many(&all_keys).await?;
        assert_eq!(delete_results.len(), 4);

        // First two should return true (existed), last two should return false (didn't exist)
        assert!(delete_results[0]);
        assert!(delete_results[1]);
        assert!(!delete_results[2]);
        assert!(!delete_results[3]);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_delete_same_node_different_checkpoints() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"versioned_node".to_vec();
        let checkpoints = vec![100u64, 200u64, 300u64];
        let values = vec![
            b"value_v1".to_vec(),
            b"value_v2".to_vec(),
            b"value_v3".to_vec(),
        ];

        // Create multiple versions of the same node
        let keys: Vec<Vec<u8>> = checkpoints
            .iter()
            .map(|&checkpoint| unchop_table_key(&node_uuid, checkpoint))
            .collect();

        // Set all versions
        for (key, value) in keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Delete only the middle version (checkpoint 200)
        let middle_key = unchop_table_key(&node_uuid, 200);
        let delete_result = store.delete(&middle_key).await?;
        assert!(delete_result);

        // Verify only the middle version is gone
        let v1_exists = store.get_exact_if_exists(&keys[0]).await?;
        let v2_exists = store.get_exact_if_exists(&keys[1]).await?;
        let v3_exists = store.get_exact_if_exists(&keys[2]).await?;

        assert_eq!(v1_exists, Some(values[0].clone()));
        assert_eq!(v2_exists, None);
        assert_eq!(v3_exists, Some(values[2].clone()));

        // Test time-travel query should now skip the deleted version
        let query_key = unchop_table_key(&node_uuid, 250); // Should find v1 (checkpoint 100)
        let result = store.get_leq(&query_key, 8).await?;
        assert_eq!(result, Some(values[0].clone()));

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_delete_empty_batch() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let empty_keys: Vec<Vec<u8>> = vec![];

        // Delete empty batch should not error
        let delete_results = store.delete_many(&empty_keys).await?;
        assert_eq!(delete_results.len(), 0);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_delete_after_time_travel() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"time_travel_delete_node".to_vec();
        let value_v1 = b"value_checkpoint_100".to_vec();
        let value_v2 = b"value_checkpoint_200".to_vec();

        let key_v1 = unchop_table_key(&node_uuid, 100);
        let key_v2 = unchop_table_key(&node_uuid, 200);

        // Set two versions
        store.set(key_v1.clone(), value_v1.clone()).await?;
        store.set(key_v2.clone(), value_v2.clone()).await?;

        // Test time travel before deletion
        let query_key = unchop_table_key(&node_uuid, 150);
        let result_before = store.get_leq(&query_key, 8).await?;
        assert_eq!(result_before, Some(value_v1.clone()));

        // Delete the earlier version
        let delete_result = store.delete(&key_v1).await?;
        assert!(delete_result);

        // Test time travel after deletion - should now return None for the same query
        let result_after = store.get_leq(&query_key, 8).await?;
        assert_eq!(result_after, None);

        // But querying at a later checkpoint should still find v2
        let query_key_later = unchop_table_key(&node_uuid, 250);
        let result_later = store.get_leq(&query_key_later, 8).await?;
        assert_eq!(result_later, Some(value_v2));

        config.cleanup().await?;
        Ok(())
    }
}
