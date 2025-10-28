use kvq::traits::KVQBinaryStoreAsync;
use psy_store::store::scylla::checkpoint_store::{
    chop_table_key, unchop_table_key, ScyllaCheckpointStore,
};
use std::sync::Arc;

mod common;
use common::*;

#[cfg(test)]
mod checkpoint_fuzzy_tests_fixed {
    use super::*;

    #[tokio::test]
    async fn test_checkpoint_store_fuzzy_query_standard_size() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"fuzzy_test_node_standard".to_vec();
        let checkpoint_ids = vec![100u64, 200u64, 300u64];
        let values = vec![
            b"value_at_100".to_vec(),
            b"value_at_200".to_vec(),
            b"value_at_300".to_vec(),
        ];

        // Set up test data
        for (&checkpoint_id, value) in checkpoint_ids.iter().zip(values.iter()) {
            let key = unchop_table_key(&node_uuid, checkpoint_id);
            store.set(key, value.clone()).await?;
        }

        // Test fuzzy query with standard fuzzy_bytes = 8 (CHECKPOINT_ID_FUZZY_SIZE)
        let query_key = unchop_table_key(&node_uuid, 150); // Between 100 and 200
        let result = store.get_leq(&query_key, 8).await?;
        assert_eq!(result, Some(values[0].clone())); // Should find checkpoint 100

        let query_key_250 = unchop_table_key(&node_uuid, 250); // Between 200 and 300
        let result_250 = store.get_leq(&query_key_250, 8).await?;
        assert_eq!(result_250, Some(values[1].clone())); // Should find checkpoint 200

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_exact_match_query() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"exact_match_test".to_vec();
        let checkpoint_id = 100u64;
        let value = b"exact_match_value".to_vec();

        let key = unchop_table_key(&node_uuid, checkpoint_id);
        store.set(key.clone(), value.clone()).await?;

        // Test exact match query
        let result = store.get_exact_if_exists(&key).await?;
        assert_eq!(result, Some(value.clone()));

        // Test exact match with get_leq and fuzzy_bytes = 0
        let result_leq = store.get_leq(&key, 0).await?;
        assert_eq!(result_leq, Some(value));

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_time_travel_with_fuzzy() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        // Create data with predictable checkpoint patterns
        let node_uuid = b"time_travel_fuzzy__".to_vec(); // 16 bytes exactly
        let checkpoint_ids = vec![
            1000u64, // 0x03E8
            2000u64, // 0x07D0
            3000u64, // 0x0BB8
            4000u64, // 0x0FA0
            5000u64, // 0x1388
        ];
        let values = generate_test_values(5);

        // Set up test data
        let keys: Vec<Vec<u8>> = checkpoint_ids
            .iter()
            .map(|&checkpoint| unchop_table_key(&node_uuid, checkpoint))
            .collect();

        for (key, value) in keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Test time travel queries at various points
        let test_cases = vec![
            (1500u64, Some(values[0].clone())), // Should find 1000
            (2500u64, Some(values[1].clone())), // Should find 2000
            (3500u64, Some(values[2].clone())), // Should find 3000
            (4500u64, Some(values[3].clone())), // Should find 4000
            (5500u64, Some(values[4].clone())), // Should find 5000
            (500u64, None),                     // Before any data
        ];

        for (query_checkpoint, expected) in test_cases {
            let query_key = unchop_table_key(&node_uuid, query_checkpoint);
            let result = store.get_leq(&query_key, 8).await?;
            assert_eq!(
                result, expected,
                "Failed for checkpoint {}",
                query_checkpoint
            );
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_get_fuzzy_range_leq_kv() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"fuzzy_range_test_".to_vec();
        let checkpoint_ids = vec![100u64, 200u64, 300u64, 400u64, 500u64];
        let values = generate_test_values(5);

        // Set up test data
        let keys: Vec<Vec<u8>> = checkpoint_ids
            .iter()
            .map(|&checkpoint| unchop_table_key(&node_uuid, checkpoint))
            .collect();

        for (key, value) in keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Test get_fuzzy_range_leq_kv
        let query_key = unchop_table_key(&node_uuid, 350); // Should find checkpoints <= 300
        let results = store.get_fuzzy_range_leq_kv(&query_key, 8).await?;

        // Should return results in descending order by checkpoint_id
        assert!(results.len() >= 3);

        // Verify that results are in descending order and contain expected data
        let mut found_300 = false;
        let mut found_200 = false;
        let mut found_100 = false;

        for result in &results {
            let (node_uuid_result, checkpoint_result) = chop_table_key(&result.key);
            assert_eq!(node_uuid_result, node_uuid);

            if checkpoint_result == 300 {
                found_300 = true;
                assert_eq!(result.value, values[2]);
            } else if checkpoint_result == 200 {
                found_200 = true;
                assert_eq!(result.value, values[1]);
            } else if checkpoint_result == 100 {
                found_100 = true;
                assert_eq!(result.value, values[0]);
            }
        }

        assert!(found_300, "Should find checkpoint 300");
        assert!(found_200, "Should find checkpoint 200");
        assert!(found_100, "Should find checkpoint 100");

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_fuzzy_range_empty_result() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"fuzzy_empty_test_".to_vec();

        // Query without any data
        let query_key = unchop_table_key(&node_uuid, 100);
        let results = store.get_fuzzy_range_leq_kv(&query_key, 8).await?;
        assert_eq!(results.len(), 0);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_fuzzy_different_node_uuids() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        // Create data for different node UUIDs
        let node_uuid_a = b"fuzzy_node_aaaa_".to_vec();
        let node_uuid_b = b"fuzzy_node_bbbb_".to_vec();
        let checkpoint_id = 100u64;
        let value_a = b"value_for_node_a".to_vec();
        let value_b = b"value_for_node_b".to_vec();

        let key_a = unchop_table_key(&node_uuid_a, checkpoint_id);
        let key_b = unchop_table_key(&node_uuid_b, checkpoint_id);

        store.set(key_a.clone(), value_a.clone()).await?;
        store.set(key_b.clone(), value_b.clone()).await?;

        // Query for node A should only return node A's value
        let result_a = store.get_leq(&key_a, 8).await?;
        assert_eq!(result_a, Some(value_a));

        // Query for node B should only return node B's value
        let result_b = store.get_leq(&key_b, 8).await?;
        assert_eq!(result_b, Some(value_b));

        // Query with non-existent node UUID should return None
        let node_uuid_c = b"fuzzy_node_cccc_".to_vec();
        let key_c = unchop_table_key(&node_uuid_c, checkpoint_id);
        let result_c = store.get_leq(&key_c, 8).await?;
        assert_eq!(result_c, None);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_leq_kv_pairs() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"test_leq_kv_node".to_vec();
        let checkpoint_id = 500u64;
        let value = b"test_leq_kv_value".to_vec();

        let full_key = unchop_table_key(&node_uuid, checkpoint_id);
        store.set(full_key.clone(), value.clone()).await?;

        // Test get_leq_kv returns both key and value
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
    async fn test_checkpoint_store_consistent_ordering() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"ordering_test_node".to_vec();

        // Insert checkpoints in non-sequential order to test ordering
        let checkpoint_data = vec![
            (300u64, b"value_300".to_vec()),
            (100u64, b"value_100".to_vec()),
            (500u64, b"value_500".to_vec()),
            (200u64, b"value_200".to_vec()),
            (400u64, b"value_400".to_vec()),
        ];

        // Set all data
        for (checkpoint_id, value) in &checkpoint_data {
            let key = unchop_table_key(&node_uuid, *checkpoint_id);
            store.set(key, value.clone()).await?;
        }

        // Test that get_leq consistently finds the correct "latest before" value
        let test_queries = vec![
            (150u64, Some(b"value_100".to_vec())), // Should find 100
            (250u64, Some(b"value_200".to_vec())), // Should find 200
            (350u64, Some(b"value_300".to_vec())), // Should find 300
            (450u64, Some(b"value_400".to_vec())), // Should find 400
            (550u64, Some(b"value_500".to_vec())), // Should find 500
            (50u64, None),                         // Before any data
        ];

        for (query_checkpoint, expected) in test_queries {
            let query_key = unchop_table_key(&node_uuid, query_checkpoint);
            let result = store.get_leq(&query_key, 8).await?;
            assert_eq!(
                result, expected,
                "Failed for query checkpoint {}",
                query_checkpoint
            );
        }

        config.cleanup().await?;
        Ok(())
    }
}
