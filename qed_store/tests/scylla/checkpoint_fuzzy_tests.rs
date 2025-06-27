use kvq::traits::{KVQBinaryStoreReaderAsync, KVQBinaryStoreWriterImmutableAsync, KVQPair};
use qed_store::store::scylla::checkpoint_store::{
    chop_table_key, unchop_table_key, ScyllaCheckpointStore,
};
use std::sync::Arc;

mod common;
use common::*;

#[cfg(test)]
mod checkpoint_fuzzy_tests {
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
            store.imm_set(key, value.clone()).await?;
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
    async fn test_checkpoint_store_fuzzy_query_zero_fuzzy_bytes() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"fuzzy_test_zero".to_vec();
        let checkpoint_id = 100u64;
        let value = b"exact_match_value".to_vec();

        let key = unchop_table_key(&node_uuid, checkpoint_id);
        store.imm_set(key.clone(), value.clone()).await?;

        // Test exact match with fuzzy_bytes = 0
        let result = store.get_leq(&key, 0).await?;
        assert_eq!(result, Some(value.clone()));

        // Test non-exact match with fuzzy_bytes = 0 should find nothing
        let different_key = unchop_table_key(&node_uuid, 200);
        let result_no_match = store.get_leq(&different_key, 0).await?;
        assert_eq!(result_no_match, None);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_fuzzy_query_variable_fuzzy_bytes() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        // Create keys with different patterns
        // Format: node_uuid (16 bytes) + checkpoint_id (8 bytes) = 24 bytes total
        let base_uuid = b"fuzzy_var_test__".to_vec(); // 16 bytes
        let checkpoint_ids = vec![
            0x1000000000000000u64, // High bit pattern
            0x2000000000000000u64,
            0x3000000000000000u64,
        ];
        let values = generate_test_values(3);

        // Set up test data
        let keys: Vec<Vec<u8>> = checkpoint_ids
            .iter()
            .map(|&checkpoint| unchop_table_key(&base_uuid, checkpoint))
            .collect();

        for (key, value) in keys.iter().zip(values.iter()) {
            store.imm_set(key.clone(), value.clone()).await?;
        }

        // Test with fuzzy_bytes = 4 (should match on 4 bytes of checkpoint)
        let query_checkpoint = 0x1500000000000000u64; // Between first and second
        let query_key = unchop_table_key(&base_uuid, query_checkpoint);
        let result_4 = store.get_leq(&query_key, 4).await?;
        assert_eq!(result_4, Some(values[0].clone()));

        // Test with fuzzy_bytes = 2 (should match on 2 bytes of checkpoint)
        let query_checkpoint_2 = 0x1100000000000000u64; // Still should match first
        let query_key_2 = unchop_table_key(&base_uuid, query_checkpoint_2);
        let result_2 = store.get_leq(&query_key_2, 2).await?;
        assert_eq!(result_2, Some(values[0].clone()));

        // Test with fuzzy_bytes = 1 (should match on 1 byte of checkpoint)
        let query_checkpoint_1 = 0x1F00000000000000u64; // Should still match first
        let query_key_1 = unchop_table_key(&base_uuid, query_checkpoint_1);
        let result_1 = store.get_leq(&query_key_1, 1).await?;
        assert_eq!(result_1, Some(values[0].clone()));

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
            store.imm_set(key.clone(), value.clone()).await?;
        }

        // Test get_fuzzy_range_leq_kv
        let query_key = unchop_table_key(&node_uuid, 350); // Should find checkpoints <= 300
        let results = store.get_fuzzy_range_leq_kv(&query_key, 8).await?;

        // Should return results in descending order by checkpoint_id
        assert!(results.len() >= 3);

        // Check that the first few results are in descending order by checkpoint_id
        // Should get checkpoints 300, 200, 100
        let first_three_results = &results[0..3.min(results.len())];
        let expected_keys = vec![
            unchop_table_key(&node_uuid, 300),
            unchop_table_key(&node_uuid, 200),
            unchop_table_key(&node_uuid, 100),
        ];
        let expected_values = vec![
            values[2].clone(), // checkpoint 300
            values[1].clone(), // checkpoint 200
            values[0].clone(), // checkpoint 100
        ];

        for (i, result) in first_three_results.iter().enumerate() {
            assert_eq!(result.key, expected_keys[i]);
            assert_eq!(result.value, expected_values[i]);
        }

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

        store.imm_set(key_a.clone(), value_a.clone()).await?;
        store.imm_set(key_b.clone(), value_b.clone()).await?;

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
    async fn test_checkpoint_store_fuzzy_bytes_edge_cases() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"fuzzy_edge_test__".to_vec();
        let checkpoint_id = 0x123456789ABCDEFu64;
        let value = b"edge_case_value".to_vec();

        let key = unchop_table_key(&node_uuid, checkpoint_id);
        store.imm_set(key.clone(), value.clone()).await?;

        // Test fuzzy_bytes larger than key size
        let result_large = store.get_leq(&key, 100).await?;
        assert_eq!(result_large, Some(value.clone()));

        // Test fuzzy_bytes equal to full key size
        let result_full = store.get_leq(&key, key.len()).await?;
        assert_eq!(result_full, Some(value.clone()));

        // Test with maximum practical fuzzy_bytes for checkpoint (8 bytes)
        let result_max = store.get_leq(&key, 8).await?;
        assert_eq!(result_max, Some(value));

        config.cleanup().await?;
        Ok(())
    }
}
