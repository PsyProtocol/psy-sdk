#[cfg(test)]
mod sync_tests {
    use std::sync::Arc;

    use crate::{
        cache::{sync_cache::KVQBinaryStoreCached, KVQBinaryStoreCachedTrait},
        memory::simple::KVQSimpleMemoryBackingStore,
        traits::KVQBinaryStore,
    };

    #[test]
    fn test_basic_operations() {
        let backing = Arc::new(KVQSimpleMemoryBackingStore::new());
        let cache = KVQBinaryStoreCached::new(backing.clone());

        cache.set_ref(&vec![1], &vec![10]).unwrap();
        assert_eq!(cache.get_exact(&vec![1]).unwrap(), vec![10]);

        assert_eq!(cache.get_exact_if_exists(&vec![1]).unwrap(), Some(vec![10]));
        assert_eq!(cache.get_exact_if_exists(&vec![2]).unwrap(), None);

        assert!(cache.delete(&vec![1]).unwrap());
        assert!(cache.get_exact(&vec![1]).is_err());
        assert_eq!(cache.get_exact_if_exists(&vec![1]).unwrap(), None);
    }

    #[test]
    fn test_get_leq_basic() {
        let backing = Arc::new(KVQSimpleMemoryBackingStore::new());
        backing.set_ref(&vec![1], &vec![10]).unwrap();
        backing.set_ref(&vec![3], &vec![30]).unwrap();
        backing.set_ref(&vec![5], &vec![50]).unwrap();

        let cache = KVQBinaryStoreCached::new(backing);

        assert_eq!(cache.get_leq(&vec![3], 0).unwrap(), Some(vec![30]));

        assert_eq!(cache.get_leq(&vec![4], 0).unwrap(), Some(vec![30]));

        assert_eq!(cache.get_leq(&vec![0], 0).unwrap(), None);

        assert_eq!(cache.get_leq(&vec![6], 0).unwrap(), Some(vec![50]));
    }

    #[test]
    fn test_get_leq_with_cache_updates() {
        let backing = Arc::new(KVQSimpleMemoryBackingStore::new());
        backing.set_ref(&vec![1], &vec![10]).unwrap();
        backing.set_ref(&vec![5], &vec![50]).unwrap();

        let cache = KVQBinaryStoreCached::new(backing);

        cache.set_ref(&vec![3], &vec![30]).unwrap();

        assert_eq!(cache.get_leq(&vec![4], 0).unwrap(), Some(vec![30]));

        assert!(cache.delete(&vec![3]).unwrap());

        assert_eq!(cache.get_leq(&vec![4], 0).unwrap(), Some(vec![10]));
    }

    #[test]
    fn test_get_fuzzy_range_leq_kv_interleaved() {
        let backing = Arc::new(KVQSimpleMemoryBackingStore::new());
        backing.set_ref(&vec![2], &vec![20]).unwrap();
        backing.set_ref(&vec![4], &vec![40]).unwrap();
        backing.set_ref(&vec![5], &vec![50]).unwrap();

        let store_result = backing.get_fuzzy_range_leq_kv(&vec![5], 0).unwrap();
        println!(
            "Store result for fuzzy_bytes=0: {:?}",
            store_result.iter().map(|kv| &kv.key).collect::<Vec<_>>()
        );

        let cache = KVQBinaryStoreCached::new(backing);
        cache.set_ref(&vec![1], &vec![10]).unwrap();
        cache.set_ref(&vec![3], &vec![30]).unwrap();

        let result = cache.get_fuzzy_range_leq_kv(&vec![5], 0).unwrap();
        println!(
            "Cache result for fuzzy_bytes=0: {:?}",
            result.iter().map(|kv| &kv.key).collect::<Vec<_>>()
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, vec![5]);
        assert_eq!(result[0].value, vec![50]);
    }

    #[test]
    fn test_get_fuzzy_range_leq_kv_with_fuzzy() {
        let backing = Arc::new(KVQSimpleMemoryBackingStore::new());
        backing.set_ref(&vec![1, 0], &vec![10]).unwrap();
        backing.set_ref(&vec![1, 1], &vec![11]).unwrap();
        backing.set_ref(&vec![1, 2], &vec![12]).unwrap();
        backing.set_ref(&vec![2, 0], &vec![20]).unwrap();

        let cache = KVQBinaryStoreCached::new(backing.clone());

        let result = cache.get_fuzzy_range_leq_kv(&vec![1, 3], 1).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].key, vec![1, 0]);
        assert_eq!(result[1].key, vec![1, 1]);
        assert_eq!(result[2].key, vec![1, 2]);

        cache.delete(&vec![1, 1]).unwrap();
        cache.set_ref(&vec![1, 3], &vec![13]).unwrap();

        let result2 = cache.get_fuzzy_range_leq_kv(&vec![1, 3], 1).unwrap();
        assert_eq!(result2.len(), 3);
        assert_eq!(result2[0].key, vec![1, 0]);
        assert_eq!(result2[1].key, vec![1, 2]);
        assert_eq!(result2[2].key, vec![1, 3]);
    }

    #[test]
    fn test_get_fuzzy_range_leq_kv_with_deletion() {
        let backing = Arc::new(KVQSimpleMemoryBackingStore::new());
        backing.set_ref(&vec![2], &vec![20]).unwrap();
        backing.set_ref(&vec![3], &vec![30]).unwrap();

        let cache = KVQBinaryStoreCached::new(backing);
        cache.delete(&vec![2]).unwrap();
        cache.set_ref(&vec![2], &vec![25]).unwrap();

        let result = cache.get_fuzzy_range_leq_kv(&vec![2], 0).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, vec![2]);
        assert_eq!(result[0].value, vec![25]);

        cache.delete(&vec![2]).unwrap();
        let result2 = cache.get_fuzzy_range_leq_kv(&vec![2], 0).unwrap();
        assert_eq!(result2.len(), 0);
    }

    #[test]
    fn test_get_fuzzy_range_leq_kv_edge_case() {
        let backing = Arc::new(KVQSimpleMemoryBackingStore::new());
        backing.set_ref(&vec![1], &vec![10]).unwrap();
        backing.set_ref(&vec![5], &vec![50]).unwrap();

        let cache = KVQBinaryStoreCached::new(backing);
        cache.set_ref(&vec![3], &vec![30]).unwrap();

        let result = cache.get_fuzzy_range_leq_kv(&vec![4], 0).unwrap();
        println!("Result for key=4: {:?}", result.iter().map(|kv| (&kv.key, &kv.value)).collect::<Vec<_>>());

        assert_eq!(result.len(), 0);

        let leq_result = cache.get_leq(&vec![4], 0).unwrap();
        assert_eq!(leq_result, Some(vec![30]));
    }

    #[test]
    fn test_get_leq_kv() {
        let backing = Arc::new(KVQSimpleMemoryBackingStore::new());
        backing.set_ref(&vec![1], &vec![10]).unwrap();
        backing.set_ref(&vec![3], &vec![30]).unwrap();

        let cache = KVQBinaryStoreCached::new(backing);

        let result = cache.get_leq_kv(&vec![2], 0).unwrap();
        assert!(result.is_some());
        let pair = result.unwrap();
        assert_eq!(pair.key, vec![1]);
        assert_eq!(pair.value, vec![10]);
    }

    #[test]
    fn test_cache_overrides_backing() {
        let backing = Arc::new(KVQSimpleMemoryBackingStore::new());
        backing.set_ref(&vec![1], &vec![10]).unwrap();

        let cache = KVQBinaryStoreCached::new(backing);

        cache.set_ref(&vec![1], &vec![20]).unwrap();

        assert_eq!(cache.get_exact(&vec![1]).unwrap(), vec![20]);

        assert_eq!(cache.get_leq(&vec![1], 0).unwrap(), Some(vec![20]));
    }

    #[test]
    fn test_fuzzy_bytes() {
        let backing = Arc::new(KVQSimpleMemoryBackingStore::new());
        backing.set_ref(&vec![1, 2, 3], &vec![10]).unwrap();
        backing.set_ref(&vec![1, 2, 5], &vec![20]).unwrap();
        backing.set_ref(&vec![1, 3, 0], &vec![30]).unwrap();

        let cache = KVQBinaryStoreCached::new(backing);

        assert_eq!(cache.get_leq(&vec![1, 2, 4], 0).unwrap(), Some(vec![10]));

        assert_eq!(cache.get_leq(&vec![1, 2, 4], 1).unwrap(), Some(vec![10]));
    }

    #[test]
    fn test_flush_changes() {
        let backing = Arc::new(KVQSimpleMemoryBackingStore::new());
        backing.set_ref(&vec![1], &vec![10]).unwrap();

        let cache = KVQBinaryStoreCached::new(backing.clone());

        cache.set_ref(&vec![2], &vec![20]).unwrap();
        cache.set_ref(&vec![3], &vec![30]).unwrap();
        cache.delete(&vec![1]).unwrap();

        let (puts, deletes) = cache.flush_changes().unwrap();

        assert_eq!(puts.len(), 2);
        assert_eq!(deletes.len(), 1);
        assert_eq!(deletes[0], vec![1]);

        for pair in puts {
            backing.set_ref(&pair.key, &pair.value).unwrap();
        }
        for k in deletes {
            assert!(backing.delete(&k).unwrap());
        }

        assert!(backing.get_exact(&vec![1]).is_err());
        assert_eq!(backing.get_exact(&vec![2]).unwrap(), vec![20]);
        assert_eq!(backing.get_exact(&vec![3]).unwrap(), vec![30]);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod async_tests {
    use std::sync::Arc;

    use super::super::test_helpers::test_helpers::AsyncMemoryStore;
    use crate::{
        cache::{async_cache::KVQBinaryStoreCachedAsync, KVQBinaryStoreCachedTraitAsync},
        traits::{KVQBinaryStore, KVQBinaryStoreAsync},
    };

    #[tokio::test]
    async fn test_basic_operations_async() {
        let backing = Arc::new(AsyncMemoryStore::new());
        let cache = KVQBinaryStoreCachedAsync::new(backing.clone());

        cache.set_ref(&vec![1], &vec![10]).await.unwrap();
        assert_eq!(cache.get_exact(&vec![1]).await.unwrap(), vec![10]);

        assert_eq!(cache.get_exact_if_exists(&vec![1]).await.unwrap(), Some(vec![10]));
        assert_eq!(cache.get_exact_if_exists(&vec![2]).await.unwrap(), None);

        assert!(cache.delete(&vec![1]).await.unwrap());
        assert!(cache.get_exact(&vec![1]).await.is_err());
        assert_eq!(cache.get_exact_if_exists(&vec![1]).await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_get_leq_basic_async() {
        let backing = Arc::new(AsyncMemoryStore::with_data(vec![
            (vec![1], vec![10]),
            (vec![3], vec![30]),
            (vec![5], vec![50]),
        ]));

        let cache = KVQBinaryStoreCachedAsync::new(backing);

        assert_eq!(cache.get_leq(&vec![3], 0).await.unwrap(), Some(vec![30]));

        assert_eq!(cache.get_leq(&vec![4], 0).await.unwrap(), Some(vec![30]));

        assert_eq!(cache.get_leq(&vec![0], 0).await.unwrap(), None);

        assert_eq!(cache.get_leq(&vec![6], 0).await.unwrap(), Some(vec![50]));
    }

    #[tokio::test]
    async fn test_get_leq_with_cache_updates_async() {
        let backing = Arc::new(AsyncMemoryStore::with_data(vec![(vec![1], vec![10]), (vec![5], vec![50])]));

        let cache = KVQBinaryStoreCachedAsync::new(backing);

        cache.set_ref(&vec![3], &vec![30]).await.unwrap();

        assert_eq!(cache.get_leq(&vec![4], 0).await.unwrap(), Some(vec![30]));

        assert!(cache.delete(&vec![3]).await.unwrap());

        assert_eq!(cache.get_leq(&vec![4], 0).await.unwrap(), Some(vec![10]));
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        use tokio::task;

        let backing = Arc::new(AsyncMemoryStore::new());
        let cache = Arc::new(KVQBinaryStoreCachedAsync::new(backing));

        let cache1 = cache.clone();
        let t1 = task::spawn(async move {
            for i in 0..10 {
                cache1.set_ref(&vec![i], &vec![i * 10]).await.unwrap();
            }
        });

        let cache2 = cache.clone();
        let t2 = task::spawn(async move {
            for i in 10..20 {
                cache2.set_ref(&vec![i], &vec![i * 10]).await.unwrap();
            }
        });

        t1.await.unwrap();
        t2.await.unwrap();

        for i in 0..20 {
            assert_eq!(cache.get_exact(&vec![i]).await.unwrap(), vec![i * 10]);
        }
    }
}
