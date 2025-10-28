use anyhow::Result;
use kvq::traits::{KVQBinaryStoreAsync, KVQPair};
use psy_store::store::tikv::{TiKVConfig, TiKVStore};

async fn create_test_store() -> Result<TiKVStore> {
    let config = TiKVConfig::default();
    TiKVStore::new(config).await
}

fn generate_test_keys(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|_| format!("test_key_{}", rand::random::<u64>()).into_bytes())
        .collect()
}

fn generate_test_values(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|_| format!("test_value_{}", rand::random::<u64>()).into_bytes())
        .collect()
}

#[cfg(test)]
mod basic_operations {
    use super::*;

    #[tokio::test]
    async fn test_get_exact_if_exists() -> Result<()> {
        let store = create_test_store().await?;
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        // Test non-existent key
        let result = store.get_exact_if_exists(&key).await?;
        assert_eq!(result, None);

        // Set key and test again
        store.set(key.clone(), value.clone()).await?;
        let result = store.get_exact_if_exists(&key).await?;
        assert_eq!(result, Some(value));

        // Clean up
        store.delete(&key).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_exact() -> Result<()> {
        let store = create_test_store().await?;
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        // Set key
        store.set(key.clone(), value.clone()).await?;
        
        // Test get_exact
        let result = store.get_exact(&key).await?;
        assert_eq!(result, value);

        // Clean up
        store.delete(&key).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_many_exact() -> Result<()> {
        let store = create_test_store().await?;
        let keys = generate_test_keys(3);
        let values = generate_test_values(3);

        // Set multiple keys
        for (key, value) in keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Test get_many_exact
        let results = store.get_many_exact(&keys).await?;
        assert_eq!(results, values);

        // Clean up
        store.delete_many(&keys).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_set_and_set_ref() -> Result<()> {
        let store = create_test_store().await?;
        let key1 = b"test_key1".to_vec();
        let value1 = b"test_value1".to_vec();
        let key2 = b"test_key2".to_vec();
        let value2 = b"test_value2".to_vec();

        // Test set
        store.set(key1.clone(), value1.clone()).await?;
        let result1 = store.get_exact(&key1).await?;
        assert_eq!(result1, value1);

        // Test set_ref
        store.set_ref(&key2, &value2).await?;
        let result2 = store.get_exact(&key2).await?;
        assert_eq!(result2, value2);

        // Clean up
        store.delete_many(&[key1, key2]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_and_delete_many() -> Result<()> {
        let store = create_test_store().await?;
        let keys = generate_test_keys(3);
        let values = generate_test_values(3);

        // Set keys
        for (key, value) in keys.iter().zip(values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Test delete single key
        let deleted = store.delete(&keys[0]).await?;
        assert!(deleted);
        
        // Verify key is deleted
        let result = store.get_exact_if_exists(&keys[0]).await?;
        assert_eq!(result, None);

        // Test delete non-existent key
        let deleted = store.delete(&keys[0]).await?;
        assert!(deleted);

        // Test delete_many
        let remaining_keys = &keys[1..];
        let deleted_results = store.delete_many(remaining_keys).await?;
        assert_eq!(deleted_results, vec![true, true]);

        // Verify all keys are deleted
        for key in remaining_keys {
            let result = store.get_exact_if_exists(key).await?;
            assert_eq!(result, None);
        }

        Ok(())
    }
}

#[cfg(test)]
mod batch_operations {
    use super::*;

    #[tokio::test]
    async fn test_set_many_vec() -> Result<()> {
        let store = create_test_store().await?;
        let items = vec![
            KVQPair {
                key: b"batch_key1".to_vec(),
                value: b"batch_value1".to_vec(),
            },
            KVQPair {
                key: b"batch_key2".to_vec(),
                value: b"batch_value2".to_vec(),
            },
            KVQPair {
                key: b"batch_key3".to_vec(),
                value: b"batch_value3".to_vec(),
            },
        ];

        // Test set_many_vec
        store.set_many_vec(items.clone()).await?;

        // Verify all items were set
        for item in &items {
            let result = store.get_exact(&item.key).await?;
            assert_eq!(result, item.value);
        }

        // Clean up
        let keys: Vec<Vec<u8>> = items.iter().map(|item| item.key.clone()).collect();
        store.delete_many(&keys).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_set_many_ref() -> Result<()> {
        let store = create_test_store().await?;
        let keys = generate_test_keys(3);
        let values = generate_test_values(3);
        
        let items: Vec<KVQPair<&Vec<u8>, &Vec<u8>>> = keys
            .iter()
            .zip(values.iter())
            .map(|(k, v)| KVQPair { key: k, value: v })
            .collect();

        // Test set_many_ref
        store.set_many_ref(&items).await?;

        // Verify all items were set
        for (key, value) in keys.iter().zip(values.iter()) {
            let result = store.get_exact(key).await?;
            assert_eq!(result, *value);
        }

        // Clean up
        store.delete_many(&keys).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_set_many_split_ref() -> Result<()> {
        let store = create_test_store().await?;
        let keys = generate_test_keys(3);
        let values = generate_test_values(3);

        // Test set_many_split_ref
        store.set_many_split_ref(&keys, &values).await?;

        // Verify all items were set
        for (key, value) in keys.iter().zip(values.iter()) {
            let result = store.get_exact(key).await?;
            assert_eq!(result, *value);
        }

        // Clean up
        store.delete_many(&keys).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_set_and_delete_many() -> Result<()> {
        let store = create_test_store().await?;
        let set_keys = generate_test_keys(3);
        let set_values = generate_test_values(3);
        let delete_keys = generate_test_keys(2);
        
        // First set some keys to delete
        for (key, value) in delete_keys.iter().zip(generate_test_values(2).iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        let set_items: Vec<KVQPair<&Vec<u8>, &Vec<u8>>> = set_keys
            .iter()
            .zip(set_values.iter())
            .map(|(k, v)| KVQPair { key: k, value: v })
            .collect();

        // Test set_and_delete_many
        store.set_and_delete_many(&set_items, &delete_keys).await?;

        // Verify set operations
        for (key, value) in set_keys.iter().zip(set_values.iter()) {
            let result = store.get_exact(key).await?;
            assert_eq!(result, *value);
        }

        // Verify delete operations
        for key in &delete_keys {
            let result = store.get_exact_if_exists(key).await?;
            assert_eq!(result, None);
        }

        // Clean up
        store.delete_many(&set_keys).await?;
        Ok(())
    }
}

#[cfg(test)]
mod fuzzy_search_operations {
    use super::*;

    /// Helper function to setup test data for fuzzy search
    async fn setup_fuzzy_test_data(store: &TiKVStore) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let test_data = vec![
            (vec![1, 1, 1], vec![11, 11, 11]),
            (vec![1, 1, 2], vec![11, 12, 12]),
            (vec![1, 1, 3], vec![11, 13, 13]),
            (vec![1, 2, 1], vec![12, 21, 21]),
            (vec![1, 2, 2], vec![12, 22, 22]),
            (vec![2, 1, 1], vec![21, 11, 11]),
        ];

        for (key, value) in &test_data {
            store.set(key.clone(), value.clone()).await?;
        }

        Ok(test_data)
    }

    #[tokio::test]
    async fn test_get_leq() -> Result<()> {
        let store = create_test_store().await?;
        let test_data = setup_fuzzy_test_data(&store).await?;

        // Test get_leq with fuzzy_bytes = 0 (exact match)
        let search_key = vec![1, 1, 2];
        let result = store.get_leq(&search_key, 0).await?;
        assert_eq!(result, Some(vec![11, 12, 12]));

        // Test get_leq with fuzzy_bytes = 1 (prefix match)
        let search_key = vec![1, 1, 4];
        let result = store.get_leq(&search_key, 1).await?;
        assert_eq!(result, Some(vec![11, 13, 13])); // Should find [1,1,3]

        // Test get_leq with non-existent key
        let search_key = vec![0, 0, 0];
        let result = store.get_leq(&search_key, 1).await?;
        assert_eq!(result, None);

        // Clean up
        for (key, _) in &test_data {
            store.delete(key).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_get_leq_kv() -> Result<()> {
        let store = create_test_store().await?;
        let test_data = setup_fuzzy_test_data(&store).await?;

        // Test get_leq_kv with exact match
        let search_key = vec![1, 1, 2];
        let result = store.get_leq_kv(&search_key, 0).await?;
        assert!(result.is_some());
        let kv_pair = result.unwrap();
        assert_eq!(kv_pair.key, vec![1, 1, 2]);
        assert_eq!(kv_pair.value, vec![11, 12, 12]);

        // Test get_leq_kv with fuzzy match
        let search_key = vec![1, 1, 4];
        let result = store.get_leq_kv(&search_key, 1).await?;
        assert!(result.is_some());
        let kv_pair = result.unwrap();
        assert_eq!(kv_pair.key, vec![1, 1, 3]);
        assert_eq!(kv_pair.value, vec![11, 13, 13]);

        // Clean up
        for (key, _) in &test_data {
            store.delete(key).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_get_fuzzy_range_leq_kv() -> Result<()> {
        let store = create_test_store().await?;
        let test_data = setup_fuzzy_test_data(&store).await?;

        // Test get_fuzzy_range_leq_kv with prefix [1,1]
        let search_key = vec![1, 1, 4];
        let result = store.get_fuzzy_range_leq_kv(&search_key, 1).await?;
        
        // Should return all keys with prefix [1,1] that are <= [1,1,4]
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].key, vec![1, 1, 1]);
        assert_eq!(result[1].key, vec![1, 1, 2]);
        assert_eq!(result[2].key, vec![1, 1, 3]);

        // Test with different fuzzy_bytes
        let search_key = vec![1, 3, 0];
        let result = store.get_fuzzy_range_leq_kv(&search_key, 2).await?;
        // Should return all keys with prefix [1] that are <= [1,3,0]
        dbg!(&result);
        assert!(result.len() >= 5); // Should find keys [1,1,1], [1,1,2], [1,1,3], [1,2,1], [1,2,2]

        // Clean up
        for (key, _) in &test_data {
            store.delete(key).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_get_many_leq() -> Result<()> {
        let store = create_test_store().await?;
        let test_data = setup_fuzzy_test_data(&store).await?;

        let search_keys = vec![
            vec![1, 1, 2], // exact match
            vec![1, 1, 4], // fuzzy match should find [1,1,3]
            vec![0, 0, 0], // no match
        ];

        let results = store.get_many_leq(&search_keys, 1).await?;
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Some(vec![11, 12, 12]));
        assert_eq!(results[1], Some(vec![11, 13, 13]));
        assert_eq!(results[2], None);

        // Clean up
        for (key, _) in &test_data {
            store.delete(key).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_get_many_leq_kv() -> Result<()> {
        let store = create_test_store().await?;
        let test_data = setup_fuzzy_test_data(&store).await?;

        let search_keys = vec![
            vec![1, 1, 2], // exact match
            vec![1, 1, 4], // fuzzy match should find [1,1,3]
            vec![0, 0, 0], // no match
        ];

        let results = store.get_many_leq_kv(&search_keys, 1).await?;
        assert_eq!(results.len(), 3);
        
        // Check first result
        assert!(results[0].is_some());
        let kv1 = results[0].as_ref().unwrap();
        assert_eq!(kv1.key, vec![1, 1, 2]);
        assert_eq!(kv1.value, vec![11, 12, 12]);
        
        // Check second result
        assert!(results[1].is_some());
        let kv2 = results[1].as_ref().unwrap();
        assert_eq!(kv2.key, vec![1, 1, 3]);
        assert_eq!(kv2.value, vec![11, 13, 13]);
        
        // Check third result
        assert!(results[2].is_none());

        // Clean up
        for (key, _) in &test_data {
            store.delete(key).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_get_leq_u() -> Result<()> {
        let store = create_test_store().await?;
        let test_data = setup_fuzzy_test_data(&store).await?;

        // Test get_leq_u (unwrapped version)
        let search_key = vec![1, 1, 2];
        let result = store.get_leq_u(&search_key, 0).await?;
        assert_eq!(result, vec![11, 12, 12]);

        // Clean up
        for (key, _) in &test_data {
            store.delete(key).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_get_leq_kv_u() -> Result<()> {
        let store = create_test_store().await?;
        let test_data = setup_fuzzy_test_data(&store).await?;

        // Test get_leq_kv_u (unwrapped version)
        let search_key = vec![1, 1, 2];
        let result = store.get_leq_kv_u(&search_key, 0).await?;
        assert_eq!(result.key, vec![1, 1, 2]);
        assert_eq!(result.value, vec![11, 12, 12]);

        // Clean up
        for (key, _) in &test_data {
            store.delete(key).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_get_many_leq_u() -> Result<()> {
        let store = create_test_store().await?;
        let test_data = setup_fuzzy_test_data(&store).await?;

        let search_keys = vec![
            vec![1, 1, 2], // exact match
            vec![1, 1, 3], // exact match
        ];

        let results = store.get_many_leq_u(&search_keys, 0).await?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], vec![11, 12, 12]);
        assert_eq!(results[1], vec![11, 13, 13]);

        // Clean up
        for (key, _) in &test_data {
            store.delete(key).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_get_many_leq_kv_u() -> Result<()> {
        let store = create_test_store().await?;
        let test_data = setup_fuzzy_test_data(&store).await?;

        let search_keys = vec![
            vec![1, 1, 2], // exact match
            vec![1, 1, 3], // exact match
        ];

        let results = store.get_many_leq_kv_u(&search_keys, 0).await?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].key, vec![1, 1, 2]);
        assert_eq!(results[0].value, vec![11, 12, 12]);
        assert_eq!(results[1].key, vec![1, 1, 3]);
        assert_eq!(results[1].value, vec![11, 13, 13]);

        // Clean up
        for (key, _) in &test_data {
            store.delete(key).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod large_data_chunking_tests {
    use super::*;

    /// Generate large data (5MB) for chunking tests
    fn generate_large_data(size_mb: usize) -> Vec<u8> {
        let size_bytes = size_mb * 1024 * 1024;
        let mut data = Vec::with_capacity(size_bytes);
        for i in 0..size_bytes {
            data.push((i % 256) as u8);
        }
        data
    }

    #[tokio::test]
    async fn test_large_data_basic_operations() -> Result<()> {
        let store = create_test_store().await?;
        let key = b"large_data_key".to_vec();
        let large_value = generate_large_data(5); // 5MB data
        
        // Test set large data
        store.set(key.clone(), large_value.clone()).await?;
        
        // Test get_exact_if_exists with large data
        let result = store.get_exact_if_exists(&key).await?;
        assert_eq!(result, Some(large_value.clone()));
        
        // Test get_exact with large data
        let result = store.get_exact(&key).await?;
        assert_eq!(result, large_value);
        
        // Clean up
        store.delete(&key).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_large_data_set_and_set_ref() -> Result<()> {
        let store = create_test_store().await?;
        let key1 = b"large_key1".to_vec();
        let key2 = b"large_key2".to_vec();
        let large_value1 = generate_large_data(6); // 6MB data
        let large_value2 = generate_large_data(7); // 7MB data

        // Test set with large data
        store.set(key1.clone(), large_value1.clone()).await?;
        let result1 = store.get_exact(&key1).await?;
        assert_eq!(result1, large_value1);

        // Test set_ref with large data
        store.set_ref(&key2, &large_value2).await?;
        let result2 = store.get_exact(&key2).await?;
        assert_eq!(result2, large_value2);

        // Clean up
        store.delete_many(&[key1, key2]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_large_data_get_many_exact() -> Result<()> {
        let store = create_test_store().await?;
        let keys = vec![
            b"large_key1".to_vec(),
            b"large_key2".to_vec(),
            b"large_key3".to_vec(),
        ];
        let large_values = vec![
            generate_large_data(5), // 5MB
            generate_large_data(6), // 6MB
            generate_large_data(7), // 7MB
        ];

        // Set multiple large data items
        for (key, value) in keys.iter().zip(large_values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Test get_many_exact with large data
        let results = store.get_many_exact(&keys).await?;
        assert_eq!(results, large_values);

        // Clean up
        store.delete_many(&keys).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_large_data_set_many_ref() -> Result<()> {
        let store = create_test_store().await?;
        let keys = vec![
            b"batch_large_key1".to_vec(),
            b"batch_large_key2".to_vec(),
            b"batch_large_key3".to_vec(),
        ];
        let large_values = vec![
            generate_large_data(5), // 5MB
            generate_large_data(6), // 6MB
            generate_large_data(7), // 7MB
        ];
        
        let items: Vec<KVQPair<&Vec<u8>, &Vec<u8>>> = keys
            .iter()
            .zip(large_values.iter())
            .map(|(k, v)| KVQPair { key: k, value: v })
            .collect();

        // Test set_many_ref with large data
        store.set_many_ref(&items).await?;

        // Verify all large data items were set correctly
        for (key, value) in keys.iter().zip(large_values.iter()) {
            let result = store.get_exact(key).await?;
            assert_eq!(result, *value);
        }

        // Clean up
        store.delete_many(&keys).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_large_data_set_many_vec() -> Result<()> {
        let store = create_test_store().await?;
        let items = vec![
            KVQPair {
                key: b"batch_large_vec_key1".to_vec(),
                value: generate_large_data(5), // 5MB
            },
            KVQPair {
                key: b"batch_large_vec_key2".to_vec(),
                value: generate_large_data(6), // 6MB
            },
            KVQPair {
                key: b"batch_large_vec_key3".to_vec(),
                value: generate_large_data(7), // 7MB
            },
        ];

        // Test set_many_vec with large data
        store.set_many_vec(items.clone()).await?;

        // Verify all large data items were set correctly
        for item in &items {
            let result = store.get_exact(&item.key).await?;
            assert_eq!(result, item.value);
        }

        // Clean up
        let keys: Vec<Vec<u8>> = items.iter().map(|item| item.key.clone()).collect();
        store.delete_many(&keys).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_large_data_set_many_split_ref() -> Result<()> {
        let store = create_test_store().await?;
        let keys = vec![
            b"split_large_key1".to_vec(),
            b"split_large_key2".to_vec(),
            b"split_large_key3".to_vec(),
        ];
        let large_values = vec![
            generate_large_data(5), // 5MB
            generate_large_data(6), // 6MB
            generate_large_data(7), // 7MB
        ];

        // Test set_many_split_ref with large data
        store.set_many_split_ref(&keys, &large_values).await?;

        // Verify all large data items were set correctly
        for (key, value) in keys.iter().zip(large_values.iter()) {
            let result = store.get_exact(key).await?;
            assert_eq!(result, *value);
        }

        // Clean up
        store.delete_many(&keys).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_large_data_delete() -> Result<()> {
        let store = create_test_store().await?;
        let key = b"large_delete_key".to_vec();
        let large_value = generate_large_data(8); // 8MB data
        
        // Set large data
        store.set(key.clone(), large_value).await?;
        
        // Verify it exists
        let result = store.get_exact_if_exists(&key).await?;
        assert!(result.is_some());
        
        // Test delete with large data
        let deleted = store.delete(&key).await?;
        assert!(deleted);
        
        // Verify it's deleted
        let result = store.get_exact_if_exists(&key).await?;
        assert_eq!(result, None);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_large_data_delete_many() -> Result<()> {
        let store = create_test_store().await?;
        let keys = vec![
            b"large_delete_key1".to_vec(),
            b"large_delete_key2".to_vec(),
            b"large_delete_key3".to_vec(),
        ];
        let large_values = vec![
            generate_large_data(5), // 5MB
            generate_large_data(6), // 6MB
            generate_large_data(7), // 7MB
        ];

        // Set multiple large data items
        for (key, value) in keys.iter().zip(large_values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Verify they exist
        let results = store.get_many_exact(&keys).await?;
        assert_eq!(results.len(), 3);

        // Test delete_many with large data
        let deleted_results = store.delete_many(&keys).await?;
        assert_eq!(deleted_results, vec![true, true, true]);

        // Verify all are deleted
        for key in &keys {
            let result = store.get_exact_if_exists(key).await?;
            assert_eq!(result, None);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_large_data_set_and_delete_many() -> Result<()> {
        let store = create_test_store().await?;
        
        // First set some large data to delete later
        let delete_keys = vec![
            b"large_delete_set_key1".to_vec(),
            // b"large_delete_set_key2".to_vec(),
        ];
        let delete_values = vec![
            generate_large_data(5), // 5MB
            // generate_large_data(6), // 6MB
        ];

        for (key, value) in delete_keys.iter().zip(delete_values.iter()) {
            store.set(key.clone(), value.clone()).await?;
        }

        // Prepare new large data to set
        let set_keys = vec![
            b"large_new_set_key1".to_vec(),
        ];
        let set_values = vec![
            generate_large_data(11), // 11MB
        ];

        let set_items: Vec<KVQPair<&Vec<u8>, &Vec<u8>>> = set_keys
            .iter()
            .zip(set_values.iter())
            .map(|(k, v)| KVQPair { key: k, value: v })
            .collect();

        // Test set_and_delete_many with large data
        store.set_and_delete_many(&set_items, &delete_keys).await?;

        // Verify set operations worked
        for (key, value) in set_keys.iter().zip(set_values.iter()) {
            let result = store.get_exact(key).await?;
            assert_eq!(result, *value);
        }

        // Verify delete operations worked
        for key in &delete_keys {
            let result = store.get_exact_if_exists(key).await?;
            assert_eq!(result, None);
        }

        // Clean up
        store.delete_many(&set_keys).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_mixed_small_and_large_data() -> Result<()> {
        let store = create_test_store().await?;
        
        // Mix of small and large data
        let items = vec![
            KVQPair {
                key: b"small_key1".to_vec(),
                value: b"small_value1".to_vec(),
            },
            KVQPair {
                key: b"large_key1".to_vec(),
                value: generate_large_data(5), // 5MB
            },
            KVQPair {
                key: b"small_key2".to_vec(),
                value: b"small_value2".to_vec(),
            },
            KVQPair {
                key: b"large_key2".to_vec(),
                value: generate_large_data(6), // 6MB
            },
        ];

        // Test set_many_vec with mixed data
        store.set_many_vec(items.clone()).await?;

        // Verify all items were set correctly
        for item in &items {
            let result = store.get_exact(&item.key).await?;
            assert_eq!(result, item.value);
        }

        // Test get_many_exact with mixed data
        let keys: Vec<Vec<u8>> = items.iter().map(|item| item.key.clone()).collect();
        let results = store.get_many_exact(&keys).await?;
        assert_eq!(results.len(), 4);
        
        for (expected, actual) in items.iter().zip(results.iter()) {
            assert_eq!(expected.value, *actual);
        }

        // Clean up
        store.delete_many(&keys).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_large_data_fuzzy_search() -> Result<()> {
        let store = create_test_store().await?;
        
        // Set up test data with large values
        let test_data = vec![
            (vec![1, 1, 1], generate_large_data(5)), // 5MB
            (vec![1, 1, 2], generate_large_data(6)), // 6MB
            (vec![1, 1, 3], generate_large_data(7)), // 7MB
            (vec![1, 2, 1], b"small_value".to_vec()),
            (vec![1, 2, 2], generate_large_data(4)), // 4MB
        ];

        for (key, value) in &test_data {
            store.set(key.clone(), value.clone()).await?;
        }

        // Test get_leq with large data
        let search_key = vec![1, 1, 2];
        let result = store.get_leq(&search_key, 0).await?;
        assert_eq!(result, Some(generate_large_data(6)));

        // Test get_leq_kv with large data
        let result = store.get_leq_kv(&search_key, 0).await?;
        assert!(result.is_some());
        let kv_pair = result.unwrap();
        assert_eq!(kv_pair.key, vec![1, 1, 2]);
        assert_eq!(kv_pair.value, generate_large_data(6));

        // Test get_many_leq with large data
        let search_keys = vec![vec![1, 1, 2], vec![1, 1, 4]];
        let results = store.get_many_leq(&search_keys, 1).await?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], Some(generate_large_data(6)));
        assert_eq!(results[1], Some(generate_large_data(7)));

        // Clean up
        for (key, _) in &test_data {
            store.delete(key).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_large_data_chunking_edge_cases() -> Result<()> {
        let store = create_test_store().await?;
        
        // Test exactly 4MB (boundary case)
        let key_exact_4mb = b"exact_4mb_key".to_vec();
        let value_exact_4mb = generate_large_data(4); // Exactly 4MB
        store.set(key_exact_4mb.clone(), value_exact_4mb.clone()).await?;
        let result = store.get_exact(&key_exact_4mb).await?;
        assert_eq!(result, value_exact_4mb);
        
        // Test just over 4MB (should be chunked)
        let key_over_4mb = b"over_4mb_key".to_vec();
        let mut value_over_4mb = generate_large_data(4);
        value_over_4mb.extend_from_slice(&vec![0u8; 1]); // 4MB + 1 byte
        store.set(key_over_4mb.clone(), value_over_4mb.clone()).await?;
        let result = store.get_exact(&key_over_4mb).await?;
        assert_eq!(result, value_over_4mb);
        
        // Test very large data (10MB)
        let key_very_large = b"very_large_key".to_vec();
        let value_very_large = generate_large_data(10); // 10MB
        store.set(key_very_large.clone(), value_very_large.clone()).await?;
        let result = store.get_exact(&key_very_large).await?;
        assert_eq!(result, value_very_large);
        
        // Clean up
        store.delete_many(&[key_exact_4mb, key_over_4mb, key_very_large]).await?;
        Ok(())
    }
}

#[cfg(test)]
mod edge_cases_and_error_handling {
    use super::*;

    #[tokio::test]
    async fn test_empty_keys_and_values() -> Result<()> {
        let store = create_test_store().await?;
        
        // Test empty key
        let empty_key = vec![];
        let value = b"test_value".to_vec();
        store.set(empty_key.clone(), value.clone()).await?;
        let result = store.get_exact(&empty_key).await?;
        assert_eq!(result, value);
        
        // Test empty value
        let key = b"test_key".to_vec();
        let empty_value = vec![];
        store.set(key.clone(), empty_value.clone()).await?;
        let result = store.get_exact(&key).await?;
        assert_eq!(result, empty_value);
        
        // Clean up
        store.delete(&empty_key).await?;
        store.delete(&key).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_large_keys_and_values() -> Result<()> {
        let store = create_test_store().await?;
        
        // Test large key (1KB)
        let large_key = vec![42u8; 1024];
        let value = b"test_value".to_vec();
        store.set(large_key.clone(), value.clone()).await?;
        let result = store.get_exact(&large_key).await?;
        assert_eq!(result, value);
        
        // Test large value (10KB)
        let key = b"test_key".to_vec();
        let large_value = vec![123u8; 10240];
        store.set(key.clone(), large_value.clone()).await?;
        let result = store.get_exact(&key).await?;
        assert_eq!(result, large_value);
        
        // Clean up
        store.delete(&large_key).await?;
        store.delete(&key).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_non_existent_key_operations() -> Result<()> {
        let store = create_test_store().await?;
        let non_existent_key = b"non_existent_key".to_vec();
        
        // Test get_exact_if_exists with non-existent key
        let result = store.get_exact_if_exists(&non_existent_key).await?;
        assert_eq!(result, None);
        
        // Test delete with non-existent key
        let deleted = store.delete(&non_existent_key).await?;
        assert!(deleted);
        
        // Test get_leq with non-existent key
        let result = store.get_leq(&non_existent_key, 1).await?;
        assert_eq!(result, None);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_batch_operations_with_empty_collections() -> Result<()> {
        let store = create_test_store().await?;
        
        // Test set_many_vec with empty vector
        let empty_items: Vec<KVQPair<Vec<u8>, Vec<u8>>> = vec![];
        store.set_many_vec(empty_items).await?;
        
        // Test get_many_exact with empty keys
        let empty_keys: Vec<Vec<u8>> = vec![];
        let results = store.get_many_exact(&empty_keys).await?;
        assert_eq!(results.len(), 0);
        
        // Test delete_many with empty keys
        let deleted_results = store.delete_many(&empty_keys).await?;
        assert_eq!(deleted_results.len(), 0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_overwrite_existing_keys() -> Result<()> {
        let store = create_test_store().await?;
        let key = b"test_key".to_vec();
        let value1 = b"value1".to_vec();
        let value2 = b"value2".to_vec();
        
        // Set initial value
        store.set(key.clone(), value1.clone()).await?;
        let result = store.get_exact(&key).await?;
        assert_eq!(result, value1);
        
        // Overwrite with new value
        store.set(key.clone(), value2.clone()).await?;
        let result = store.get_exact(&key).await?;
        assert_eq!(result, value2);
        
        // Clean up
        store.delete(&key).await?;
        Ok(())
    }
}

#[cfg(test)]
mod performance_and_stress_tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_performance() -> Result<()> {
        let store = create_test_store().await?;
        let batch_size = 100;
        
        // Generate test data
        let items: Vec<KVQPair<Vec<u8>, Vec<u8>>> = (0..batch_size)
            .map(|i| KVQPair {
                key: format!("perf_key_{:04}", i).into_bytes(),
                value: format!("perf_value_{:04}", i).into_bytes(),
            })
            .collect();
        
        // Test batch set
        let start = std::time::Instant::now();
        store.set_many_vec(items.clone()).await?;
        let set_duration = start.elapsed();
        println!("Batch set of {} items took: {:?}", batch_size, set_duration);
        
        // Test batch get
        let keys: Vec<Vec<u8>> = items.iter().map(|item| item.key.clone()).collect();
        let start = std::time::Instant::now();
        let results = store.get_many_exact(&keys).await?;
        let get_duration = start.elapsed();
        println!("Batch get of {} items took: {:?}", batch_size, get_duration);
        
        assert_eq!(results.len(), batch_size);
        
        // Test batch delete
        let start = std::time::Instant::now();
        let deleted = store.delete_many(&keys).await?;
        let delete_duration = start.elapsed();
        println!("Batch delete of {} items took: {:?}", batch_size, delete_duration);
        
        assert_eq!(deleted.len(), batch_size);
        assert!(deleted.iter().all(|&d| d));
        
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_operations() -> Result<()> {
        let store = create_test_store().await?;
        let num_tasks = 10;
        let items_per_task = 10;
        
        // Create concurrent tasks
        let mut handles = vec![];
        
        for task_id in 0..num_tasks {
            let store_clone = store.clone();
            let handle = tokio::spawn(async move {
                let mut task_items = vec![];
                
                // Each task works with its own set of keys
                for i in 0..items_per_task {
                    let key = format!("concurrent_{}_{}", task_id, i).into_bytes();
                    let value = format!("value_{}_{}", task_id, i).into_bytes();
                    task_items.push(KVQPair { key, value });
                }
                
                // Set items
                store_clone.set_many_vec(task_items.clone()).await?;
                
                // Get items to verify
                let keys: Vec<Vec<u8>> = task_items.iter().map(|item| item.key.clone()).collect();
                let results = store_clone.get_many_exact(&keys).await?;
                
                // Verify results
                for (item, result) in task_items.iter().zip(results.iter()) {
                    assert_eq!(item.value, *result);
                }
                
                // Clean up
                store_clone.delete_many(&keys).await?;
                
                anyhow::Ok(())
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await??;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_workflow() -> Result<()> {
        let store = create_test_store().await?;
        
        // Step 1: Set up initial data
        let initial_items = vec![
            KVQPair { key: b"user:1".to_vec(), value: b"alice".to_vec() },
            KVQPair { key: b"user:2".to_vec(), value: b"bob".to_vec() },
            KVQPair { key: b"user:3".to_vec(), value: b"charlie".to_vec() },
        ];
        store.set_many_vec(initial_items.clone()).await?;
        
        // Step 2: Query individual items
        for item in &initial_items {
            let result = store.get_exact(&item.key).await?;
            assert_eq!(result, item.value);
        }
        
        // Step 3: Batch query
        let keys: Vec<Vec<u8>> = initial_items.iter().map(|item| item.key.clone()).collect();
        let results = store.get_many_exact(&keys).await?;
        assert_eq!(results.len(), 3);
        
        // Step 4: Update some items
        let updates = vec![
            KVQPair { key: b"user:1".to_vec(), value: b"alice_updated".to_vec() },
            KVQPair { key: b"user:4".to_vec(), value: b"david".to_vec() },
        ];
        store.set_many_vec(updates.clone()).await?;
        
        // Step 5: Verify updates
        let result = store.get_exact(&b"user:1".to_vec()).await?;
        assert_eq!(result, b"alice_updated".to_vec());
        let result = store.get_exact(&b"user:4".to_vec()).await?;
        assert_eq!(result, b"david".to_vec());
        
        // Step 6: Delete some items
        let delete_keys = vec![b"user:2".to_vec(), b"user:3".to_vec()];
        let deleted = store.delete_many(&delete_keys).await?;
        assert_eq!(deleted, vec![true, true]);
        
        // Step 7: Verify deletions
        for key in &delete_keys {
            let result = store.get_exact_if_exists(key).await?;
            assert_eq!(result, None);
        }
        
        // Step 8: Final cleanup
        let remaining_keys = vec![b"user:1".to_vec(), b"user:4".to_vec()];
        store.delete_many(&remaining_keys).await?;
        
        Ok(())
    }
}