#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::cache::async_cache::KVQBinaryStoreCachedAsync;
    use crate::cache::{CacheValueType, KVQBinaryStoreCachedTraitAsync};
    use crate::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};
    use crate::memory::simple::KVQSimpleMemoryBackingStore;
    use std::sync::Arc;

    fn create_test_store() -> KVQBinaryStoreCachedAsync<KVQSimpleMemoryBackingStore> {
        let backing_store = Arc::new(KVQSimpleMemoryBackingStore::new());
        KVQBinaryStoreCachedAsync::new(backing_store)
    }

    fn setup_store_with_data() -> KVQBinaryStoreCachedAsync<KVQSimpleMemoryBackingStore> {
        let backing_store = Arc::new(KVQSimpleMemoryBackingStore::new());
        
        // Add some initial data to backing store
        backing_store.put(&vec![1], &vec![10]).unwrap();
        backing_store.put(&vec![3], &vec![30]).unwrap();
        backing_store.put(&vec![5], &vec![50]).unwrap();
        backing_store.put(&vec![7], &vec![70]).unwrap();
        backing_store.put(&vec![9], &vec![90]).unwrap();
        
        KVQBinaryStoreCachedAsync::new(backing_store)
    }

    #[tokio::test]
    async fn test_basic_get_put_async() {
        let store = create_test_store();
        
        // Test put and get
        store.put(&vec![1, 2, 3], &vec![4, 5, 6]).await.unwrap();
        let result = store.get_exact(&vec![1, 2, 3]).await.unwrap();
        assert_eq!(result, vec![4, 5, 6]);
        
        // Test get from cache
        let result2 = store.get_exact(&vec![1, 2, 3]).await.unwrap();
        assert_eq!(result2, vec![4, 5, 6]);
    }

    #[tokio::test]
    async fn test_get_exact_if_exists_async() {
        let store = create_test_store();
        
        // Test non-existent key
        let result = store.get_exact_if_exists(&vec![1, 2, 3]).await.unwrap();
        assert_eq!(result, None);
        
        // Add key
        store.put(&vec![1, 2, 3], &vec![4, 5, 6]).await.unwrap();
        
        // Test existing key
        let result = store.get_exact_if_exists(&vec![1, 2, 3]).await.unwrap();
        assert_eq!(result, Some(vec![4, 5, 6]));
    }

    #[tokio::test]
    async fn test_delete_async() {
        let store = setup_store_with_data();
        
        // Delete a key
        store.delete(&vec![3]).await.unwrap();
        
        // Verify it's marked as removed in cache
        assert!(store.is_removed(&vec![3]).await);
        
        // Verify get returns error
        let result = store.get_exact(&vec![3]).await;
        assert!(result.is_err());
        
        // Verify get_exact_if_exists returns None
        let result = store.get_exact_if_exists(&vec![3]).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_leq_basic_async() {
        let store = setup_store_with_data();
        
        // Test exact match
        let result = store.get_leq(&vec![5]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![5], value: vec![50] }));
        
        // Test less than
        let result = store.get_leq(&vec![4]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![3], value: vec![30] }));
        
        // Test less than smallest
        let result = store.get_leq(&vec![0]).await.unwrap();
        assert_eq!(result, None);
        
        // Test greater than largest
        let result = store.get_leq(&vec![10]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![9], value: vec![90] }));
    }

    #[tokio::test]
    async fn test_get_leq_with_cache_modifications_async() {
        let store = setup_store_with_data();
        
        // Add new values to cache
        store.put(&vec![2], &vec![20]).await.unwrap();
        store.put(&vec![6], &vec![60]).await.unwrap();
        
        // Test with cache values
        let result = store.get_leq(&vec![6]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![6], value: vec![60] }));
        
        let result = store.get_leq(&vec![4]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![3], value: vec![30] }));
        
        // Delete a value
        store.delete(&vec![5]).await.unwrap();
        
        // Test get_leq skips deleted value
        let result = store.get_leq(&vec![5]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![3], value: vec![30] }));
        
        let result = store.get_leq(&vec![6]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![6], value: vec![60] }));
    }

    #[tokio::test]
    async fn test_get_leq_edge_cases_async() {
        let store = create_test_store();
        
        // Empty store
        let result = store.get_leq(&vec![5]).await.unwrap();
        assert_eq!(result, None);
        
        // Single element
        store.put(&vec![5], &vec![50]).await.unwrap();
        
        let result = store.get_leq(&vec![3]).await.unwrap();
        assert_eq!(result, None);
        
        let result = store.get_leq(&vec![5]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![5], value: vec![50] }));
        
        let result = store.get_leq(&vec![7]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![5], value: vec![50] }));
    }

    #[tokio::test]
    async fn test_get_leq_all_deleted_async() {
        let store = setup_store_with_data();
        
        // Delete all keys
        store.delete(&vec![1]).await.unwrap();
        store.delete(&vec![3]).await.unwrap();
        store.delete(&vec![5]).await.unwrap();
        store.delete(&vec![7]).await.unwrap();
        store.delete(&vec![9]).await.unwrap();
        
        // All get_leq should return None
        let result = store.get_leq(&vec![10]).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_ge_basic_async() {
        let store = setup_store_with_data();
        
        // Test exact match
        let result = store.get_ge(&vec![5]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![5], value: vec![50] }));
        
        // Test greater than
        let result = store.get_ge(&vec![4]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![5], value: vec![50] }));
        
        // Test greater than largest
        let result = store.get_ge(&vec![10]).await.unwrap();
        assert_eq!(result, None);
        
        // Test less than smallest
        let result = store.get_ge(&vec![0]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![1], value: vec![10] }));
    }

    #[tokio::test]
    async fn test_get_ge_with_cache_modifications_async() {
        let store = setup_store_with_data();
        
        // Add new values to cache
        store.put(&vec![2], &vec![20]).await.unwrap();
        store.put(&vec![6], &vec![60]).await.unwrap();
        
        // Delete a value
        store.delete(&vec![5]).await.unwrap();
        
        // Test get_ge skips deleted value
        let result = store.get_ge(&vec![5]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![6], value: vec![60] }));
        
        let result = store.get_ge(&vec![4]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![6], value: vec![60] }));
    }

    #[tokio::test]
    async fn test_cache_key_management_async() {
        let store = create_test_store();
        
        // Add keys
        store.put(&vec![1], &vec![10]).await.unwrap();
        store.put(&vec![2], &vec![20]).await.unwrap();
        store.put(&vec![3], &vec![30]).await.unwrap();
        
        // Delete one
        store.delete(&vec![2]).await.unwrap();
        
        // Check non-removed keys
        let non_removed = store.get_non_removed_keys().await;
        assert_eq!(non_removed.len(), 2);
        assert!(non_removed.contains(&vec![1]));
        assert!(non_removed.contains(&vec![3]));
        
        // Check removed keys
        let removed = store.get_removed_keys().await;
        assert_eq!(removed.len(), 1);
        assert!(removed.contains(&vec![2]));
    }

    #[tokio::test]
    async fn test_overwrite_deleted_key_async() {
        let store = setup_store_with_data();
        
        // Delete and then re-add key
        store.delete(&vec![3]).await.unwrap();
        assert!(store.is_removed(&vec![3]).await);
        
        store.put(&vec![3], &vec![35]).await.unwrap();
        assert!(!store.is_removed(&vec![3]).await);
        
        let result = store.get_exact(&vec![3]).await.unwrap();
        assert_eq!(result, vec![35]);
    }

    #[tokio::test]
    async fn test_get_leq_complex_scenario_async() {
        let store = create_test_store();
        
        // Create a complex scenario with backing store and cache
        let backing = Arc::clone(&store.store);
        backing.put(&vec![10], &vec![100]).unwrap();
        backing.put(&vec![20], &vec![200]).unwrap();
        backing.put(&vec![30], &vec![300]).unwrap();
        
        // Cache modifications
        store.put(&vec![15], &vec![150]).await.unwrap();  // New in cache
        store.put(&vec![25], &vec![250]).await.unwrap();  // New in cache
        store.delete(&vec![20]).await.unwrap();           // Delete from backing
        
        // Test various get_leq operations
        let result = store.get_leq(&vec![12]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![10], value: vec![100] }));
        
        let result = store.get_leq(&vec![17]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![15], value: vec![150] }));
        
        let result = store.get_leq(&vec![22]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![15], value: vec![150] })); // 20 is deleted
        
        let result = store.get_leq(&vec![27]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![25], value: vec![250] }));
    }

    #[tokio::test]
    async fn test_byte_ordering_async() {
        let store = create_test_store();
        
        // Test with multi-byte keys to ensure proper ordering
        store.put(&vec![1, 0], &vec![10]).await.unwrap();
        store.put(&vec![1, 5], &vec![15]).await.unwrap();
        store.put(&vec![2, 0], &vec![20]).await.unwrap();
        store.put(&vec![10, 0], &vec![100]).await.unwrap();
        
        let result = store.get_leq(&vec![1, 3]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![1, 0], value: vec![10] }));
        
        let result = store.get_leq(&vec![1, 10]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![1, 5], value: vec![15] }));
        
        let result = store.get_leq(&vec![5, 0]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![2, 0], value: vec![20] }));
    }

    #[tokio::test]
    async fn test_concurrent_modifications() {
        use tokio::task;
        
        let store = Arc::new(setup_store_with_data());
        
        // Spawn multiple tasks that modify the cache concurrently
        let store1 = Arc::clone(&store);
        let task1 = task::spawn(async move {
            for i in 0..10 {
                store1.put(&vec![100 + i], &vec![200 + i]).await.unwrap();
            }
        });
        
        let store2 = Arc::clone(&store);
        let task2 = task::spawn(async move {
            for i in 0..10 {
                store2.put(&vec![50 + i], &vec![150 + i]).await.unwrap();
            }
        });
        
        let store3 = Arc::clone(&store);
        let task3 = task::spawn(async move {
            // Delete some existing keys
            store3.delete(&vec![3]).await.unwrap();
            store3.delete(&vec![7]).await.unwrap();
        });
        
        // Wait for all tasks to complete
        let _ = tokio::join!(task1, task2, task3);
        
        // Verify cache consistency
        let result = store.get_leq(&vec![55]).await.unwrap();
        assert!(result.is_some());
        
        // Verify deleted keys are not returned
        let result = store.get_exact_if_exists(&vec![3]).await.unwrap();
        assert_eq!(result, None);
        
        let result = store.get_exact_if_exists(&vec![7]).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_leq_with_empty_cache_values() {
        let store = create_test_store();
        
        // Add keys with empty values
        store.put(&vec![1], &vec![]).await.unwrap();
        store.put(&vec![2], &vec![]).await.unwrap();
        store.put(&vec![3], &vec![]).await.unwrap();
        
        let result = store.get_leq(&vec![2]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![2], value: vec![] }));
    }

    #[tokio::test]
    async fn test_get_leq_boundary_conditions() {
        let store = setup_store_with_data();
        
        // Test with max value key
        let max_key = vec![255, 255, 255, 255];
        let result = store.get_leq(&max_key).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![9], value: vec![90] }));
        
        // Test with empty key
        let result = store.get_leq(&vec![]).await.unwrap();
        assert_eq!(result, None);
        
        // Add empty key
        store.put(&vec![], &vec![0]).await.unwrap();
        let result = store.get_leq(&vec![0]).await.unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![], value: vec![0] }));
    }
}