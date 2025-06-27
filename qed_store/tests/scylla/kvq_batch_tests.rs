use kvq::traits::{
    KVQBinaryStoreAsync,
    KVQPair,
};
use qed_store::store::scylla::kvq_store::ScyllaKVQStore;

mod common;
use common::*;

#[cfg(test)]
mod kvq_batch_tests {
    use super::*;

    #[tokio::test]
    async fn test_kvq_set_many_ref_small_batch() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let keys = generate_test_keys(5);
        let values = generate_test_values(5);
        let items_ref = create_kvq_pairs_ref(&keys, &values);

        // Test batch set with small batch (should use ScyllaDB batch)
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
    async fn test_kvq_set_many_ref_large_batch() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let keys = generate_test_keys(20); // Large batch > 16
        let values = generate_test_values(20);
        let items_ref = create_kvq_pairs_ref(&keys, &values);

        // Test batch set with large batch (should fall back to individual inserts)
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
    async fn test_kvq_set_many_vec() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let items = generate_test_kvpairs(8);

        // Test set_many_vec
        store.set_many_vec(items.clone()).await?;

        // Verify all items were set
        for item in &items {
            let retrieved = store.get_exact(&item.key).await?;
            assert_eq!(retrieved, item.value);
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_set_many_split_ref() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let keys = generate_test_keys(6);
        let values = generate_test_values(6);

        // Test set_many_split_ref
        store.set_many_split_ref(&keys, &values).await?;

        // Verify all items were set
        for (key, value) in keys.iter().zip(values.iter()) {
            let retrieved = store.get_exact(key).await?;
            assert_eq!(retrieved, *value);
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_set_many_split_ref_length_mismatch() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let keys = generate_test_keys(3);
        let values = generate_test_values(5); // Different length

        // Test should return error for mismatched lengths
        let result = store.set_many_split_ref(&keys, &values).await;
        assert!(result.is_err());

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_set_many_ref_empty() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let empty_items: Vec<KVQPair<&Vec<u8>, &Vec<u8>>> = vec![];

        // Test empty batch should not error
        store.set_many_ref(&empty_items).await?;

        config.cleanup().await?;
        Ok(())
    }
}

#[cfg(test)]
mod kvq_immutable_batch_tests {
    use super::*;

    #[tokio::test]
    async fn test_kvq_set_many_ref_small_batch() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let keys = generate_test_keys(10);
        let values = generate_test_values(10);
        let items_ref = create_kvq_pairs_ref(&keys, &values);

        // Test immutable batch set
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
    async fn test_kvq_set_many_ref_large_batch() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let keys = generate_test_keys(25); // Large batch
        let values = generate_test_values(25);
        let items_ref = create_kvq_pairs_ref(&keys, &values);

        // Test large immutable batch set
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
    async fn test_kvq_set_many_vec() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let items = generate_test_kvpairs(7);

        // Test immutable set_many_vec
        store.set_many_vec(items.clone()).await?;

        // Verify all items were set
        for item in &items {
            let retrieved = store.get_exact(&item.key).await?;
            assert_eq!(retrieved, item.value);
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_set_many_split_ref() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let keys = generate_test_keys(4);
        let values = generate_test_values(4);

        // Test immutable set_many_split_ref
        store.set_many_split_ref(&keys, &values).await?;

        // Verify all items were set
        for (key, value) in keys.iter().zip(values.iter()) {
            let retrieved = store.get_exact(key).await?;
            assert_eq!(retrieved, *value);
        }

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_batch_performance_comparison() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let keys = generate_test_keys(100);
        let values = generate_test_values(100);

        // Test individual sets (for comparison)
        let start = std::time::Instant::now();
        for (key, value) in keys.iter().take(50).zip(values.iter().take(50)) {
            store.set(key.clone(), value.clone()).await?;
        }
        let individual_duration = start.elapsed();

        // Test batch set
        let remaining_keys = &keys[50..];
        let remaining_values = &values[50..];
        let items_ref = create_kvq_pairs_ref(remaining_keys, remaining_values);

        let start = std::time::Instant::now();
        store.set_many_ref(&items_ref).await?;
        let batch_duration = start.elapsed();

        println!("Individual sets (50 items): {:?}", individual_duration);
        println!("Batch set (50 items): {:?}", batch_duration);

        // Verify all items were set correctly
        for (key, value) in keys.iter().zip(values.iter()) {
            let retrieved = store.get_exact(key).await?;
            assert_eq!(retrieved, *value);
        }

        config.cleanup().await?;
        Ok(())
    }
}
