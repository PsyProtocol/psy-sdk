use kvq::traits::KVQPair;
    KVQBinaryStoreAsync,
    KVQPair, KVQSerializable,
};
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_store::store::scylla::{
    checkpoint_store::{chop_table_key, unchop_table_key, ScyllaCheckpointStore},
    kvq_store::ScyllaKVQStore,
    merkle_store::ScyllaMerkleStore,
};
use qed_store::{
    models::kvq_merkle::key::KVQMerkleNodeKey,
    traits::merkle_store::{
        MerkleNodeStoreReaderImmutableAsync, MerkleNodeStoreWriterImmutableAsync,
    },
};
use std::sync::Arc;

mod common;
use common::*;

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn test_kvq_store_empty_keys_and_values() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        // Test empty key
        let empty_key = vec![];
        let value = b"value_for_empty_key".to_vec();

        store.set(empty_key.clone(), value.clone()).await?;
        let retrieved = store.get_exact(&empty_key).await?;
        assert_eq!(retrieved, value);

        // Test empty value
        let key = b"key_for_empty_value".to_vec();
        let empty_value = vec![];

        store.set(key.clone(), empty_value.clone()).await?;
        let retrieved_empty = store.get_exact(&key).await?;
        assert_eq!(retrieved_empty, empty_value);

        // Test both empty
        let empty_key2 = vec![];
        let empty_value2 = vec![];

        store.set(empty_key2.clone(), empty_value2.clone()).await?;
        let retrieved_both_empty = store.get_exact(&empty_key2).await?;
        assert_eq!(retrieved_both_empty, empty_value2);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_store_very_large_keys_and_values() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        // Note: The composite adapter adds a 2-byte table type prefix to all keys
        // So we need to account for this when testing ScyllaDB's 65535 byte key limit
        
        // Test large key (65533 bytes + 2 byte prefix = 65535 bytes total)
        let large_key = vec![0xAB; 65533];
        let value = b"value_for_large_key".to_vec();

        store.set(large_key.clone(), value.clone()).await?;
        let retrieved = store.get_exact(&large_key).await?;
        assert_eq!(retrieved, value);

        // Test key at near limit (65530 bytes + 2 byte prefix = 65532 bytes total)
        let near_max_key = vec![0xAC; 65530];
        let value2 = b"value_for_near_max_key".to_vec();

        store.set(near_max_key.clone(), value2.clone()).await?;
        let retrieved_max = store.get_exact(&near_max_key).await?;
        assert_eq!(retrieved_max, value2);

        // Test large value (1MB)
        let key = b"key_for_large_value".to_vec();
        let large_value = vec![0xCD; 1024 * 1024];

        store.set(key.clone(), large_value.clone()).await?;
        let retrieved_large = store.get_exact(&key).await?;
        assert_eq!(retrieved_large, large_value);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_store_binary_data_with_nulls() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        // Test key with null bytes
        let key_with_nulls = vec![0x00, 0x01, 0x00, 0x02, 0x00];
        let value = b"value_with_null_key".to_vec();

        store.set(key_with_nulls.clone(), value.clone()).await?;
        let retrieved = store.get_exact(&key_with_nulls).await?;
        assert_eq!(retrieved, value);

        // Test value with null bytes
        let key = b"key_for_null_value".to_vec();
        let value_with_nulls = vec![0x00, 0xFF, 0x00, 0xAA, 0x00];

        store.set(key.clone(), value_with_nulls.clone()).await?;
        let retrieved_nulls = store.get_exact(&key).await?;
        assert_eq!(retrieved_nulls, value_with_nulls);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_edge_case_checkpoints() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaCheckpointStore::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let node_uuid = b"edge_checkpoint_node".to_vec();
        let value = b"edge_checkpoint_value".to_vec();

        // Test checkpoint_id = 0
        let key_zero = unchop_table_key(&node_uuid, 0);
        store.set(key_zero.clone(), value.clone()).await?;
        let retrieved_zero = store.get_exact(&key_zero).await?;
        assert_eq!(retrieved_zero, value);

        // Test checkpoint_id = u64::MAX
        let key_max = unchop_table_key(&node_uuid, u64::MAX);
        store.set(key_max.clone(), value.clone()).await?;
        let retrieved_max = store.get_exact(&key_max).await?;
        assert_eq!(retrieved_max, value);

        // Test time travel with extreme values
        let query_key = unchop_table_key(&node_uuid, u64::MAX - 1);
        let result = store.get_leq(&query_key, 8).await?;
        assert!(result.is_some());

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoint_store_key_encoding_edge_cases() -> anyhow::Result<()> {
        // Test chop/unchop with edge case data

        // Empty node UUID
        let empty_uuid = vec![];
        let checkpoint_id = 12345u64;
        let key = unchop_table_key(&empty_uuid, checkpoint_id);
        let (chopped_uuid, chopped_checkpoint) = chop_table_key(&key);
        assert_eq!(chopped_uuid, empty_uuid);
        assert_eq!(chopped_checkpoint, checkpoint_id);

        // Very long node UUID
        let long_uuid = vec![0x42; 1000];
        let key_long = unchop_table_key(&long_uuid, checkpoint_id);
        let (chopped_long_uuid, chopped_checkpoint_long) = chop_table_key(&key_long);
        assert_eq!(chopped_long_uuid, long_uuid);
        assert_eq!(chopped_checkpoint_long, checkpoint_id);

        // Node UUID with all possible byte values
        let all_bytes_uuid: Vec<u8> = (0..=255).collect();
        let key_all_bytes = unchop_table_key(&all_bytes_uuid, checkpoint_id);
        let (chopped_all_bytes, chopped_checkpoint_all) = chop_table_key(&key_all_bytes);
        assert_eq!(chopped_all_bytes, all_bytes_uuid);
        assert_eq!(chopped_checkpoint_all, checkpoint_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_edge_case_keys() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let hash = QHashOut::from_values(1, 2, 3, 4);

        // Test with extreme values
        let edge_key = KVQMerkleNodeKey::<1> {
            tree_id: u8::MAX,
            primary_id: u64::MAX,
            secondary_id: u32::MAX,
            level: u8::MAX,
            index: u64::MAX,
            checkpoint_id: u64::MAX,
        };

        store.set_node_params(&edge_key, hash).await?;
        let retrieved = store.get_node_value_if_exists(&edge_key).await?;
        assert_eq!(retrieved, Some(hash));

        // Test with zero values
        let zero_key = KVQMerkleNodeKey::<1> {
            tree_id: 0,
            primary_id: 0,
            secondary_id: 0,
            level: 0,
            index: 0,
            checkpoint_id: 0,
        };

        store.set_node_params(&zero_key, hash).await?;
        let retrieved_zero = store.get_node_value_if_exists(&zero_key).await?;
        assert_eq!(retrieved_zero, Some(hash));

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_hash_edge_cases() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let key = KVQMerkleNodeKey::<1> {
            tree_id: 1,
            primary_id: 0,
            secondary_id: 0,
            level: 1,
            index: 1,
            checkpoint_id: 1,
        };

        // Test zero hash
        let zero_hash = QHashOut::from_values(0, 0, 0, 0);
        store.set_node_params(&key, zero_hash).await?;
        let retrieved_zero = store.get_node_value_if_exists(&key).await?;
        assert_eq!(retrieved_zero, Some(zero_hash));

        // Test max hash
        let max_hash = QHashOut::from_values(u64::MAX, u64::MAX, u64::MAX, u64::MAX);
        let key_max = KVQMerkleNodeKey::<1> { index: 2, ..key };
        store.set_node_params(&key_max, max_hash).await?;
        let retrieved_max = store.get_node_value_if_exists(&key_max).await?;
        assert_eq!(retrieved_max, Some(max_hash));

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_operations() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = Arc::new(
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?,
        );

        let mut handles = vec![];

        // Spawn multiple concurrent operations
        for i in 0..10 {
            let store_clone = store.clone();
            let handle = tokio::spawn(async move {
                let key = format!("concurrent_key_{}", i).into_bytes();
                let value = format!("concurrent_value_{}", i).into_bytes();

                // Set and get concurrently
                store_clone.set(key.clone(), value.clone()).await?;
                let retrieved = store_clone.get_exact(&key).await?;
                assert_eq!(retrieved, value);

                anyhow::Ok(())
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            handle.await??;
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_unicode_and_special_characters() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        // Test Unicode characters
        let unicode_key = "🔑_test_key_🚀".as_bytes().to_vec();
        let unicode_value = "🎯_test_value_🌟".as_bytes().to_vec();

        store
            .set(unicode_key.clone(), unicode_value.clone())
            .await?;
        let retrieved = store.get_exact(&unicode_key).await?;
        assert_eq!(retrieved, unicode_value);

        // Test various special characters
        let special_key = "!@#$%^&*()_+-=[]{}|;':\",./<>?`~".as_bytes().to_vec();
        let special_value = "äöüßÄÖÜñáéíóúSpecialCharactersTest".as_bytes().to_vec();

        store
            .set(special_key.clone(), special_value.clone())
            .await?;
        let retrieved_special = store.get_exact(&special_key).await?;
        assert_eq!(retrieved_special, special_value);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_connection_handling() -> anyhow::Result<()> {
        // Test with invalid connection string
        let config = TestConfig::new().await?;

        let result =
            ScyllaKVQStore::new("invalid_host:9999", &config.keyspace, &config.table_name).await;

        // Should fail to connect
        assert!(result.is_err());

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_batch_operations_stress() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        // Test stress with many small batches
        for batch_num in 0..10 {
            let keys: Vec<Vec<u8>> = (0..100)
                .map(|i| format!("stress_key_{}_{}", batch_num, i).into_bytes())
                .collect();
            let values: Vec<Vec<u8>> = (0..100)
                .map(|i| format!("stress_value_{}_{}", batch_num, i).into_bytes())
                .collect();

            let items_ref = create_kvq_pairs_ref(&keys, &values);

            // Batch set
            store.set_many_ref(&items_ref).await?;

            // Batch get to verify
            let retrieved = store.get_many_exact(&keys).await?;
            assert_eq!(retrieved, values);
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_fuzzy_query_edge_cases() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        // Test fuzzy query with identical prefixes but different suffixes
        let keys = vec![
            vec![0x01, 0x02, 0x03, 0x04, 0x05],
            vec![0x01, 0x02, 0x03, 0x04, 0x06],
            vec![0x01, 0x02, 0x03, 0x04, 0x07],
        ];
        let values = generate_test_values(3);

        for (key, value) in keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Test with different fuzzy_bytes values
        let search_key = vec![0x01, 0x02, 0x03, 0x04, 0x06];

        // Exact match (fuzzy_bytes = 0)
        let exact_result = store.get_leq(&search_key, 0).await?;
        assert_eq!(exact_result, Some(values[1].clone()));

        // Fuzzy match with 1 byte (should match prefix)
        let fuzzy_1_result = store.get_leq(&search_key, 1).await?;
        assert!(fuzzy_1_result.is_some());

        // Fuzzy match with more bytes than key length
        let fuzzy_large_result = store.get_leq(&search_key, 100).await?;
        assert_eq!(fuzzy_large_result, Some(values[1].clone()));

        config.cleanup().await?;
        Ok(())
    }
}
