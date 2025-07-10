use kvq::traits::KVQPair;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
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

        // Test set and get using the correct API
        store.set_node_params(&key, hash).await?;
        let retrieved = store.get_node_if_exists(&key).await?;
        assert!(retrieved.is_some());
        let retrieved_pair = retrieved.unwrap();
        assert_eq!(retrieved_pair.key, key);
        assert_eq!(retrieved_pair.value, hash);

        // Test get_node_value_if_exists
        let value = store.get_node_value_if_exists(&key).await?;
        assert_eq!(value, Some(hash));

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

        // Test get_node_values (batch get)
        let retrieved_values = store.get_node_values(&keys).await?;
        assert_eq!(retrieved_values.len(), 5);

        for (retrieved, expected) in retrieved_values.iter().zip(hashes.iter()) {
            assert_eq!(retrieved, &Some(*expected));
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_set_nodes_batch() -> anyhow::Result<()> {
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

        // Create KVQPair nodes for batch set
        let nodes: Vec<KVQPair<KVQMerkleNodeKey<1>, QHashOut<GoldilocksField>>> = keys
            .iter()
            .zip(hashes.iter())
            .map(|(key, hash)| KVQPair {
                key: *key,
                value: *hash,
            })
            .collect();

        // Test batch set
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

        // Create nodes for batch set
        let nodes: Vec<KVQPair<KVQMerkleNodeKey<1>, QHashOut<GoldilocksField>>> = keys
            .iter()
            .zip(hashes.iter())
            .map(|(key, hash)| KVQPair {
                key: *key,
                value: *hash,
            })
            .collect();

        // Test large batch set
        store.set_nodes(&nodes).await?;

        // Test large batch get
        let retrieved_values = store.get_node_values(&keys).await?;
        assert_eq!(retrieved_values.len(), 20);

        for (retrieved, expected) in retrieved_values.iter().zip(hashes.iter()) {
            assert_eq!(retrieved, &Some(*expected));
        }

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
        let retrieved_values = store.get_node_values(&keys).await?;
        for (retrieved, expected) in retrieved_values.iter().zip(hashes.iter()) {
            assert_eq!(retrieved, &Some(*expected));
        }

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
        let empty_nodes: Vec<KVQPair<KVQMerkleNodeKey<1>, QHashOut<GoldilocksField>>> = vec![];

        // Test empty batch operations should not error
        store.set_nodes(&empty_nodes).await?;
        let results = store.get_node_values(&empty_keys).await?;
        assert_eq!(results.len(), 0);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_mixed_batch_results() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let checkpoint_id = 700u64;
        let existing_keys = generate_merkle_node_keys::<1>(3, checkpoint_id);
        let hashes = generate_test_hashes(3);

        // Set only some keys
        for (key, hash) in existing_keys.iter().zip(hashes.iter()) {
            store.set_node_params(key, *hash).await?;
        }

        // Query the keys we set
        let results = store.get_node_values(&existing_keys).await?;
        assert_eq!(results.len(), 3);

        // All should match our hashes
        for i in 0..3 {
            assert_eq!(results[i], Some(hashes[i]));
        }

        // Test querying non-existent keys separately
        let nonexistent_keys = generate_merkle_node_keys::<1>(2, 999);
        let nonexistent_results = store.get_node_values(&nonexistent_keys).await?;
        assert_eq!(nonexistent_results.len(), 2);

        // Results for non-existent keys should be consistent (either None or zero hash)
        for result in &nonexistent_results {
            // Accept either None or zero hash as valid for non-existent keys
            if let Some(hash) = result {
                // If not None, verify it's a zero hash (default value)
                assert_eq!(*hash, QHashOut::from_values(0, 0, 0, 0));
            }
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_merkle_store_same_tree_optimization() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store = ScyllaMerkleStore::<QHashOut<GoldilocksField>, 1>::init(
            config.keyspace.clone(),
            config.table_name.clone(),
            config.session.clone(),
        )
        .await?;

        let checkpoint_id = 800u64;

        // Create keys with same tree_id for optimization
        let keys: Vec<KVQMerkleNodeKey<1>> = (0..5)
            .map(|i| KVQMerkleNodeKey::<1> {
                tree_id: 1, // Same tree
                primary_id: 0,
                secondary_id: 0,
                level: 1,
                index: i as u64,
                checkpoint_id,
            })
            .collect();

        let hashes = generate_test_hashes(5);

        // Create nodes for same tree batch set
        let nodes: Vec<KVQPair<KVQMerkleNodeKey<1>, QHashOut<GoldilocksField>>> = keys
            .iter()
            .zip(hashes.iter())
            .map(|(key, hash)| KVQPair {
                key: *key,
                value: *hash,
            })
            .collect();

        // Test set_nodes_same_tree (should use optimization)
        store.set_nodes_same_tree(&nodes).await?;

        // Verify all nodes were set correctly
        let retrieved_values = store.get_node_values(&keys).await?;
        for (retrieved, expected) in retrieved_values.iter().zip(hashes.iter()) {
            assert_eq!(retrieved, &Some(*expected));
        }

        config.cleanup().await?;
        Ok(())
    }
}

#[cfg(test)]
mod merkle_store_performance_tests {
    use super::*;

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
        let remaining_nodes: Vec<KVQPair<KVQMerkleNodeKey<1>, QHashOut<GoldilocksField>>> =
            remaining_keys
                .iter()
                .zip(remaining_hashes.iter())
                .map(|(key, hash)| KVQPair {
                    key: *key,
                    value: *hash,
                })
                .collect();

        let start = std::time::Instant::now();
        store.set_nodes(&remaining_nodes).await?;
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
        for (retrieved, expected) in all_retrieved.iter().zip(hashes.iter()) {
            assert_eq!(retrieved, &Some(*expected));
        }

        config.cleanup().await?;
        Ok(())
    }
}
