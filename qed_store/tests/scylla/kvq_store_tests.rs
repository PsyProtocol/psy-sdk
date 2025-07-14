use kvq::traits::{
    KVQBinaryStoreAsync,
    KVQPair,
};
use qed_store::store::scylla::kvq_store::ScyllaKVQStore;
use std::sync::Arc;

mod common;
use common::*;

#[cfg(test)]
mod kvq_basic_tests {
    use super::*;

    #[tokio::test]
    async fn test_kvq_store_creation() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;

        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        // Verify store was created successfully
        assert!(!config.keyspace.is_empty());
        assert!(!config.table_name.is_empty());

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_basic_get_set() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        // Test set and get
        store.set(key.clone(), value.clone()).await?;
        let retrieved = store.get_exact(&key).await?;
        assert_eq!(retrieved, value);

        // Test get_exact_if_exists
        let exists = store.get_exact_if_exists(&key).await?;
        assert_eq!(exists, Some(value.clone()));

        // Test non-existent key
        let non_existent_key = b"non_existent".to_vec();
        let not_found = store.get_exact_if_exists(&non_existent_key).await?;
        assert_eq!(not_found, None);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_set_ref() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        // Test set_ref
        store.set_ref(&key, &value).await?;
        let retrieved = store.get_exact(&key).await?;
        assert_eq!(retrieved, value);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_get_many_exact() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let keys = generate_test_keys(5);
        let values = generate_test_values(5);

        // Set multiple key-value pairs
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
    async fn test_kvq_delete() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        // Set, then delete
        store.set(key.clone(), value.clone()).await?;
        let exists_before = store.get_exact_if_exists(&key).await?;
        assert!(exists_before.is_some());

        let deleted = store.delete(&key).await?;
        assert!(deleted); // ScyllaDB always returns true

        let exists_after = store.get_exact_if_exists(&key).await?;
        assert!(exists_after.is_none());

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_delete_many() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let mut store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let keys = generate_test_keys(3);
        let values = generate_test_values(3);

        // Set multiple key-value pairs
        for (key, value) in keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Delete multiple keys
        let delete_results = store.delete_many(&keys).await?;
        assert_eq!(delete_results.len(), 3);
        assert!(delete_results.iter().all(|&result| result)); // All should return true

        // Verify all keys are deleted
        for key in &keys {
            let exists = store.get_exact_if_exists(key).await?;
            assert!(exists.is_none());
        }

        config.cleanup().await?;
        Ok(())
    }
}

#[cfg(test)]
mod kvq_immutable_tests {
    use super::*;

    #[tokio::test]
    async fn test_kvq_immutable_set() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        // Test immutable set operations
        store.set(key.clone(), value.clone()).await?;
        let retrieved = store.get_exact(&key).await?;
        assert_eq!(retrieved, value);

        // Test set_ref
        let key2 = b"test_key2".to_vec();
        let value2 = b"test_value2".to_vec();
        store.set_ref(&key2, &value2).await?;
        let retrieved2 = store.get_exact(&key2).await?;
        assert_eq!(retrieved2, value2);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_immutable_delete() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        // Set then delete immutably
        store.set(key.clone(), value.clone()).await?;
        let deleted = store.delete(&key).await?;
        assert!(deleted);

        let exists = store.get_exact_if_exists(&key).await?;
        assert!(exists.is_none());

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_immutable_delete_many() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        let keys = generate_test_keys(3);
        let values = generate_test_values(3);

        // Set multiple keys
        for (key, value) in keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Delete multiple keys immutably
        let delete_results = store.delete_many(&keys).await?;
        assert_eq!(delete_results.len(), 3);
        assert!(delete_results.iter().all(|&result| result));

        // Verify all deleted
        for key in &keys {
            let exists = store.get_exact_if_exists(key).await?;
            assert!(exists.is_none());
        }

        config.cleanup().await?;
        Ok(())
    }
}

#[cfg(test)]
mod kvq_fuzzy_tests {
    use super::*;

    #[tokio::test]
    async fn test_kvq_get_leq() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        // Set up test data with binary keys that can be compared
        let key1 = vec![1, 2, 3, 4, 5];
        let key2 = vec![1, 2, 3, 4, 6];
        let key3 = vec![1, 2, 3, 4, 7];
        let value1 = b"value1".to_vec();
        let value2 = b"value2".to_vec();
        let value3 = b"value3".to_vec();

        store.set(key1.clone(), value1.clone()).await?;
        store.set(key2.clone(), value2.clone()).await?;
        store.set(key3.clone(), value3.clone()).await?;

        // Test get_leq - should find the largest key <= search key
        let search_key = vec![1, 2, 3, 4, 6]; // Should match key2
        let result = store.get_leq(&search_key, 0).await?;
        assert_eq!(result, Some(value2.clone()));

        // Test get_leq_kv
        let result_kv = store.get_leq_kv(&search_key, 0).await?;
        assert!(result_kv.is_some());
        let kv_pair = result_kv.unwrap();
        assert_eq!(kv_pair.key, key2);
        assert_eq!(kv_pair.value, value2);

        config.cleanup().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_kvq_get_many_leq() -> anyhow::Result<()> {
        let config = TestConfig::new().await?;
        let store =
            ScyllaKVQStore::new(&config.uri, &config.keyspace, &config.table_name).await?;

        // Set up test data
        let keys = vec![
            vec![1, 2, 3, 4, 5],
            vec![1, 2, 3, 4, 6],
            vec![1, 2, 3, 4, 7],
        ];
        let values = vec![b"value1".to_vec(), b"value2".to_vec(), b"value3".to_vec()];

        for (key, value) in keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Test get_many_leq
        let search_keys = vec![vec![1, 2, 3, 4, 5], vec![1, 2, 3, 4, 6]];
        let results = store.get_many_leq(&search_keys, 0).await?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], Some(values[0].clone()));
        assert_eq!(results[1], Some(values[1].clone()));

        config.cleanup().await?;
        Ok(())
    }
}
