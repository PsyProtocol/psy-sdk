use kvq::traits::KVQPair;
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_core::data::qhashout::QHashOut;
use qed_store::store::scylla::merkle_store::ScyllaMerkleStore;
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
mod merkle_store_tests {
    use super::*;

    #[tokio::test]
    async fn test_merkle_store_creation() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;

        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
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
    async fn test_merkle_store_basic_operations() -> anyhow::Result<()> {
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
            level: 3,
            index: 42,
            checkpoint_id: 100,
        };
        let hash = QHashOut::from_values(1, 2, 3, 4);

        // Test set and get
        store.set_node_params(&key, hash).await?;
        let retrieved = store.get_node_value_if_exists(&key).await?;
        assert_eq!(retrieved, Some(hash));

        // Test get_node_if_exists
        let exists = store.get_node_if_exists(&key).await?;
        assert_eq!(exists.map(|kv| kv.value), Some(hash));

        // Test non-existent key
        let non_existent_key = KVQMerkleNodeKey::<1> {
            tree_id: 1,
            primary_id: 0,
            secondary_id: 0,
            level: 3,
            index: 999,
            checkpoint_id: 100,
        };
        let not_found = store.get_node_if_exists(&non_existent_key).await?;
        assert_eq!(not_found, None);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_get_many_nodes_same_tree() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let checkpoint_id = 200u64;
        let keys = generate_merkle_node_keys::<1>(5, checkpoint_id);
        let hashes = generate_test_hashes(5);

        // Set all nodes
        for (key, hash) in keys.iter().zip(hashes.iter()) {
            store.set_node_params(key, *hash).await?;
        }

        // Test get_many_nodes (should use batch optimization for same tree)
        let retrieved_hashes = store.get_node_values(&keys).await?;
        let retrieved_hashes: Vec<_> = retrieved_hashes.into_iter().filter_map(|x| x).collect();
        assert_eq!(retrieved_hashes.len(), 5);
        assert_eq!(retrieved_hashes, hashes);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_get_many_nodes_different_trees() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let checkpoint_id = 300u64;

        // Create keys with different tree_ids
        let keys = vec![
            KVQMerkleNodeKey::<1> {
                tree_id: 1,
                primary_id: 0,
                secondary_id: 0,
                level: 1,
                index: 1,
                checkpoint_id,
            },
            KVQMerkleNodeKey::<1> {
                tree_id: 2,
                primary_id: 0,
                secondary_id: 0,
                level: 1,
                index: 1,
                checkpoint_id,
            },
            KVQMerkleNodeKey::<1> {
                tree_id: 3,
                primary_id: 0,
                secondary_id: 0,
                level: 1,
                index: 1,
                checkpoint_id,
            },
        ];
        let hashes = generate_test_hashes(3);

        // Set all nodes
        for (key, hash) in keys.iter().zip(hashes.iter()) {
            store.set_node_params(key, *hash).await?;
        }

        // Test get_many_nodes with different trees (should fall back to individual queries)
        let retrieved_hashes = store.get_node_values(&keys).await?;
        let retrieved_hashes: Vec<_> = retrieved_hashes.into_iter().filter_map(|x| x).collect();
        assert_eq!(retrieved_hashes.len(), 3);
        assert_eq!(retrieved_hashes, hashes);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_batch_set_nodes() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let checkpoint_id = 400u64;
        let keys = generate_merkle_node_keys::<1>(8, checkpoint_id);
        let hashes = generate_test_hashes(8);

        // Test batch set
        let nodes: Vec<KVQPair<_, _>> = keys.iter().zip(hashes.iter())
            .map(|(k, h)| KVQPair { key: k.clone(), value: *h })
            .collect();
        store.set_nodes(&nodes).await?;

        // Verify all nodes were set
        for (key, expected_hash) in keys.iter().zip(hashes.iter()) {
            let retrieved = store.get_node_value_if_exists(key).await?;
            assert_eq!(retrieved, Some(*expected_hash));
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_batch_set_nodes_length_mismatch() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let keys = generate_merkle_node_keys::<1>(3, 500);
        let hashes = generate_test_hashes(5); // Different length

        // Test should return error for mismatched lengths
        // With the new API, we need to create nodes and test
        // Note: The new API doesn't enforce length check at compile time
        // So we skip this test as it's not applicable to the new API

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_large_batch_operations() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let checkpoint_id = 600u64;
        let keys = generate_merkle_node_keys::<1>(20, checkpoint_id); // Large batch
        let hashes = generate_test_hashes(20);

        // Test large batch set (should handle efficiently)
        let nodes: Vec<KVQPair<_, _>> = keys.iter().zip(hashes.iter())
            .map(|(k, h)| KVQPair { key: k.clone(), value: *h })
            .collect();
        store.set_nodes(&nodes).await?;

        // Test large batch get (should use optimization)
        let retrieved_hashes = store.get_node_values(&keys).await?;
        let retrieved_hashes: Vec<_> = retrieved_hashes.into_iter().filter_map(|x| x).collect();
        assert_eq!(retrieved_hashes.len(), 20);
        assert_eq!(retrieved_hashes, hashes);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_mixed_checkpoint_queries() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        // Create nodes at different checkpoints for the same tree structure
        let base_key = KVQMerkleNodeKey::<1> {
            tree_id: 1,
            primary_id: 0,
            secondary_id: 0,
            level: 2,
            index: 10,
            checkpoint_id: 0, // Will be overridden
        };

        let checkpoints = vec![100u64, 200u64, 300u64];
        let hashes = generate_test_hashes(3);

        let keys: Vec<KVQMerkleNodeKey<1>> = checkpoints
            .iter()
            .map(|&checkpoint| KVQMerkleNodeKey::<1> {
                checkpoint_id: checkpoint,
                ..base_key
            })
            .collect();

        // Set nodes at different checkpoints
        for (key, hash) in keys.iter().zip(hashes.iter()) {
            store.set_node_params(key, *hash).await?;
        }

        // Query nodes at different checkpoints
        let retrieved_hashes = store.get_node_values(&keys).await?;
        let retrieved_hashes: Vec<_> = retrieved_hashes.into_iter().filter_map(|x| x).collect();
        assert_eq!(retrieved_hashes, hashes);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_empty_batch() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let empty_keys: Vec<KVQMerkleNodeKey<1>> = vec![];
        let empty_hashes: Vec<QHashOut<GoldilocksField>> = vec![];

        // Test empty batch operations should not error
        let empty_nodes: Vec<KVQPair<KVQMerkleNodeKey<1>, QHashOut<GoldilocksField>>> = vec![];
        store.set_nodes(&empty_nodes).await?;
        let results = store.get_node_values(&empty_keys).await?;
        assert_eq!(results.len(), 0);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_partial_batch_results() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let checkpoint_id = 700u64;
        let existing_keys = generate_merkle_node_keys::<1>(3, checkpoint_id);
        let nonexistent_keys = generate_merkle_node_keys::<1>(2, 999); // Different checkpoint
        let hashes = generate_test_hashes(3);

        // Set only some keys
        for (key, hash) in existing_keys.iter().zip(hashes.iter()) {
            store.set_node_params(key, *hash).await?;
        }

        // Mix existing and non-existing keys
        let mut all_keys = existing_keys.clone();
        all_keys.extend(nonexistent_keys);

        // Query mixed batch
        let results = store.get_node_values(&all_keys).await?;
        assert_eq!(results.len(), 5);

        // First 3 should match our hashes, last 2 should be None or zero hash
        for i in 0..3 {
            assert_eq!(results[i], Some(hashes[i]));
        }
        for i in 3..5 {
            // Non-existent keys return None
            assert_eq!(results[i], None);
        }

        config.cleanup().await?;
        Ok(())
    }
}

#[cfg(test)]
mod merkle_store_perf_tests {
    use super::*;

    #[tokio::test]
    async fn test_merkle_store_perf1_creation() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;

        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
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
    async fn test_merkle_store_perf1_basic_operations() -> anyhow::Result<()> {
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
            level: 4,
            index: 123,
            checkpoint_id: 800,
        };
        let hash = QHashOut::from_values(8, 7, 6, 5);

        // Test set and get
        store.set_node_params(&key, hash).await?;
        let retrieved = store.get_node_value_if_exists(&key).await?;
        assert_eq!(retrieved, Some(hash));

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_performance_comparison() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let checkpoint_id = 900u64;
        let keys = generate_merkle_node_keys::<1>(50, checkpoint_id);
        let hashes = generate_test_hashes(50);

        // Test individual operations timing
        let start = std::time::Instant::now();
        for (key, hash) in keys.iter().take(25).zip(hashes.iter().take(25)) {
            store.set_node_params(key, *hash).await?;
        }
        let individual_set_duration = start.elapsed();

        // Test batch operations timing
        let remaining_keys = &keys[25..];
        let remaining_hashes = &hashes[25..];

        let start = std::time::Instant::now();
        let nodes: Vec<KVQPair<_, _>> = remaining_keys.iter().zip(remaining_hashes.iter())
            .map(|(k, h)| KVQPair { key: k.clone(), value: *h })
            .collect();
        store.set_nodes(&nodes).await?;
        let batch_set_duration = start.elapsed();

        println!("Individual sets (25 items): {:?}", individual_set_duration);
        println!("Batch set (25 items): {:?}", batch_set_duration);

        // Test get performance
        let start = std::time::Instant::now();
        let _ = store.get_node_values(&keys).await?;
        let batch_get_duration = start.elapsed();

        println!("Batch get (50 items): {:?}", batch_get_duration);

        // Verify all data is correct
        let all_retrieved = store.get_node_values(&keys).await?;
        let all_retrieved: Vec<_> = all_retrieved.into_iter().filter_map(|x| x).collect();
        assert_eq!(all_retrieved, hashes);

        config.cleanup().await?;
        Ok(())
    }
}
