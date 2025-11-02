#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::cache::sync_cache::KVQBinaryStoreCached;
    use crate::cache::{CacheValueType, KVQBinaryStoreCachedTrait};
    use crate::traits::{KVQBinaryStore, KVQPair};
    use crate::memory::simple::KVQSimpleMemoryBackingStore;
    use std::sync::Arc;

    fn create_test_store() -> KVQBinaryStoreCached<KVQSimpleMemoryBackingStore> {
        let backing_store = Arc::new(KVQSimpleMemoryBackingStore::new());
        KVQBinaryStoreCached::new(backing_store)
    }

    fn setup_store_with_data() -> KVQBinaryStoreCached<KVQSimpleMemoryBackingStore> {
        let backing_store = KVQSimpleMemoryBackingStore::new();
        
        // Add some initial data to backing store
        backing_store.put(&vec![1], &vec![10]).unwrap();
        backing_store.put(&vec![3], &vec![30]).unwrap();
        backing_store.put(&vec![5], &vec![50]).unwrap();
        backing_store.put(&vec![7], &vec![70]).unwrap();
        backing_store.put(&vec![9], &vec![90]).unwrap();
        
        KVQBinaryStoreCached::new(Arc::new(backing_store))
    }

    #[test]
    fn test_basic_get_put() {
        let store = create_test_store();
        
        // Test put and get
        store.put(&vec![1, 2, 3], &vec![4, 5, 6]).unwrap();
        let result = store.get_exact(&vec![1, 2, 3]).unwrap();
        assert_eq!(result, vec![4, 5, 6]);
        
        // Test get from cache
        let result2 = store.get_exact(&vec![1, 2, 3]).unwrap();
        assert_eq!(result2, vec![4, 5, 6]);
    }

    #[test]
    fn test_get_exact_if_exists() {
        let store = create_test_store();
        
        // Test non-existent key
        let result = store.get_exact_if_exists(&vec![1, 2, 3]).unwrap();
        assert_eq!(result, None);
        
        // Add key
        store.put(&vec![1, 2, 3], &vec![4, 5, 6]).unwrap();
        
        // Test existing key
        let result = store.get_exact_if_exists(&vec![1, 2, 3]).unwrap();
        assert_eq!(result, Some(vec![4, 5, 6]));
    }

    #[test]
    fn test_delete() {
        let store = setup_store_with_data();
        
        // Delete a key
        store.delete(&vec![3]).unwrap();
        
        // Verify it's marked as removed in cache
        assert!(store.is_removed(&vec![3]));
        
        // Verify get returns error
        let result = store.get_exact(&vec![3]);
        assert!(result.is_err());
        
        // Verify get_exact_if_exists returns None
        let result = store.get_exact_if_exists(&vec![3]).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_leq_basic() {
        let store = setup_store_with_data();
        
        // Test exact match
        let result = store.get_leq(&vec![5], 0).unwrap();
        assert_eq!(result, Some(vec![50]));
        
        // Test less than
        let result = store.get_leq(&vec![4], 0).unwrap();
        assert_eq!(result, Some(vec![30]));
        
        // Test less than smallest
        let result = store.get_leq(&vec![0], 0).unwrap();
        assert_eq!(result, None);
        
        // Test greater than largest
        let result = store.get_leq(&vec![10], 0).unwrap();
        assert_eq!(result, Some(vec![90]));
    }

    #[test]
    fn test_get_leq_with_cache_modifications() {
        let store = setup_store_with_data();
        
        // Add new values to cache
        store.put(&vec![2], &vec![20]).unwrap();
        store.put(&vec![6], &vec![60]).unwrap();
        
        // Test with cache values
        let result = store.get_leq(&vec![6]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![6], value: vec![60] }));
        
        let result = store.get_leq(&vec![4]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![3], value: vec![30] }));
        
        // Delete a value
        store.delete(&vec![5]).unwrap();
        
        // Test get_leq skips deleted value
        let result = store.get_leq(&vec![5]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![3], value: vec![30] }));
        
        let result = store.get_leq(&vec![6]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![6], value: vec![60] }));
    }

    #[test]
    fn test_get_leq_edge_cases() {
        let store = create_test_store();
        
        // Empty store
        let result = store.get_leq(&vec![5]).unwrap();
        assert_eq!(result, None);
        
        // Single element
        store.put(&vec![5], &vec![50]).unwrap();
        
        let result = store.get_leq(&vec![3]).unwrap();
        assert_eq!(result, None);
        
        let result = store.get_leq(&vec![5]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![5], value: vec![50] }));
        
        let result = store.get_leq(&vec![7]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![5], value: vec![50] }));
    }

    #[test]
    fn test_get_leq_all_deleted() {
        let store = setup_store_with_data();
        
        // Delete all keys
        store.delete(&vec![1]).unwrap();
        store.delete(&vec![3]).unwrap();
        store.delete(&vec![5]).unwrap();
        store.delete(&vec![7]).unwrap();
        store.delete(&vec![9]).unwrap();
        
        // All get_leq should return None
        let result = store.get_leq(&vec![10]).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_ge_basic() {
        let store = setup_store_with_data();
        
        // Test exact match
        let result = store.get_ge(&vec![5]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![5], value: vec![50] }));
        
        // Test greater than
        let result = store.get_ge(&vec![4]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![5], value: vec![50] }));
        
        // Test greater than largest
        let result = store.get_ge(&vec![10]).unwrap();
        assert_eq!(result, None);
        
        // Test less than smallest
        let result = store.get_ge(&vec![0]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![1], value: vec![10] }));
    }

    #[test]
    fn test_get_ge_with_cache_modifications() {
        let store = setup_store_with_data();
        
        // Add new values to cache
        store.put(&vec![2], &vec![20]).unwrap();
        store.put(&vec![6], &vec![60]).unwrap();
        
        // Delete a value
        store.delete(&vec![5]).unwrap();
        
        // Test get_ge skips deleted value
        let result = store.get_ge(&vec![5]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![6], value: vec![60] }));
        
        let result = store.get_ge(&vec![4]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![6], value: vec![60] }));
    }

    #[test]
    fn test_cache_key_management() {
        let store = create_test_store();
        
        // Add keys
        store.put(&vec![1], &vec![10]).unwrap();
        store.put(&vec![2], &vec![20]).unwrap();
        store.put(&vec![3], &vec![30]).unwrap();
        
        // Delete one
        store.delete(&vec![2]).unwrap();
        
        // Check non-removed keys
        let non_removed = store.get_non_removed_keys();
        assert_eq!(non_removed.len(), 2);
        assert!(non_removed.contains(&vec![1]));
        assert!(non_removed.contains(&vec![3]));
        
        // Check removed keys
        let removed = store.get_removed_keys();
        assert_eq!(removed.len(), 1);
        assert!(removed.contains(&vec![2]));
    }

    #[test]
    fn test_overwrite_deleted_key() {
        let store = setup_store_with_data();
        
        // Delete and then re-add key
        store.delete(&vec![3]).unwrap();
        assert!(store.is_removed(&vec![3]));
        
        store.put(&vec![3], &vec![35]).unwrap();
        assert!(!store.is_removed(&vec![3]));
        
        let result = store.get_exact(&vec![3]).unwrap();
        assert_eq!(result, vec![35]);
    }

    #[test]
    fn test_get_leq_complex_scenario() {
        let store = create_test_store();
        
        // Create a complex scenario with backing store and cache
        let backing = Arc::clone(&store.store);
        backing.put(&vec![10], &vec![100]).unwrap();
        backing.put(&vec![20], &vec![200]).unwrap();
        backing.put(&vec![30], &vec![300]).unwrap();
        
        // Cache modifications
        store.put(&vec![15], &vec![150]).unwrap();  // New in cache
        store.put(&vec![25], &vec![250]).unwrap();  // New in cache
        store.delete(&vec![20]).unwrap();           // Delete from backing
        
        // Test various get_leq operations
        let result = store.get_leq(&vec![12]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![10], value: vec![100] }));
        
        let result = store.get_leq(&vec![17]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![15], value: vec![150] }));
        
        let result = store.get_leq(&vec![22]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![15], value: vec![150] })); // 20 is deleted
        
        let result = store.get_leq(&vec![27]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![25], value: vec![250] }));
    }

    #[test]
    fn test_byte_ordering() {
        let store = create_test_store();
        
        // Test with multi-byte keys to ensure proper ordering
        store.put(&vec![1, 0], &vec![10]).unwrap();
        store.put(&vec![1, 5], &vec![15]).unwrap();
        store.put(&vec![2, 0], &vec![20]).unwrap();
        store.put(&vec![10, 0], &vec![100]).unwrap();
        
        let result = store.get_leq(&vec![1, 3]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![1, 0], value: vec![10] }));
        
        let result = store.get_leq(&vec![1, 10]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![1, 5], value: vec![15] }));
        
        let result = store.get_leq(&vec![5, 0]).unwrap();
        assert_eq!(result, Some(KVQPair { key: vec![2, 0], value: vec![20] }));
    }
}