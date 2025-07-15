use kvq::traits::KVQBinaryStoreAsync;
use kvq::traits::KVQPair;
use qed_store::store::scylla::kvq_store::ScyllaKVQStore;
use qed_store::store::scylla::clustering_store::ScyllaClusteringStore;
use anyhow::Result;

mod common;
use common::*;

#[tokio::test]
async fn test_kvq_store_batch_exact_15() -> Result<()> {
    let config = TestConfig::new().await?;
    let store = ScyllaKVQStore::new(&config.uri, &config.keyspace, "test_batch_exact_15").await?;
    
    let keys: Vec<Vec<u8>> = (0..15).map(|i| format!("key{:03}", i).into_bytes()).collect();
    let values: Vec<Vec<u8>> = (0..15).map(|i| format!("value{:03}", i).into_bytes()).collect();
    
    let items: Vec<_> = keys.iter().zip(values.iter())
        .map(|(k, v)| KVQPair { key: k, value: v })
        .collect();
    store.set_many_ref(&items).await?;
    
    let results = store.get_many_exact(&keys).await?;
    assert_eq!(results.len(), 15);
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result, &values[i]);
    }
    
    let delete_results = store.delete_many(&keys).await?;
    assert_eq!(delete_results.len(), 15);
    assert!(delete_results.iter().all(|&r| r));
    
    for key in &keys {
        assert!(store.get_exact_if_exists(key).await?.is_none());
    }
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_kvq_store_batch_more_than_15() -> Result<()> {
    let config = TestConfig::new().await?;
    let store = ScyllaKVQStore::new(&config.uri, &config.keyspace, "test_batch_more_15").await?;
    
    let keys: Vec<Vec<u8>> = (0..47).map(|i| format!("key{:03}", i).into_bytes()).collect();
    let values: Vec<Vec<u8>> = (0..47).map(|i| format!("value{:03}", i).into_bytes()).collect();
    
    let items: Vec<_> = keys.iter().zip(values.iter())
        .map(|(k, v)| KVQPair { key: k, value: v })
        .collect();
    store.set_many_ref(&items).await?;
    
    let results = store.get_many_exact(&keys).await?;
    assert_eq!(results.len(), 47);
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result, &values[i]);
    }
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_clustering_store_batch_exact_15() -> Result<()> {
    let config = TestConfig::new().await?;
    let store = ScyllaClusteringStore::new(&config.uri, &config.keyspace, "test_clustering_batch_15", 8).await?;
    
    let keys: Vec<Vec<u8>> = (0..15u64).map(|i| {
        let mut key = format!("prefix{:02}", i / 5).into_bytes();
        key.extend_from_slice(&i.to_be_bytes());
        key
    }).collect();
    let values: Vec<Vec<u8>> = (0..15).map(|i| format!("value{:03}", i).into_bytes()).collect();
    
    let items: Vec<_> = keys.iter().zip(values.iter())
        .map(|(k, v)| KVQPair { key: k, value: v })
        .collect();
    store.set_many_ref(&items).await?;
    
    let results = store.get_many_exact(&keys).await?;
    assert_eq!(results.len(), 15);
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result, &values[i]);
    }
    
    let delete_results = store.delete_many(&keys).await?;
    assert_eq!(delete_results.len(), 15);
    assert!(delete_results.iter().all(|&r| r));
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_kvq_store_get_many_leq_chunked() -> Result<()> {
    let config = TestConfig::new().await?;
    let store = ScyllaKVQStore::new(&config.uri, &config.keyspace, "test_get_many_leq").await?;
    
    // Store keys with pattern "keyXXXXXX" where X is a 6-digit number
    let base_keys: Vec<Vec<u8>> = (0..50).map(|i| {
        format!("key{:06}", i * 2).into_bytes()  // Store even numbers: 0, 2, 4, ..., 98
    }).collect();
    let values: Vec<Vec<u8>> = (0..50).map(|i| format!("value{:03}", i).into_bytes()).collect();
    
    let items: Vec<_> = base_keys.iter().zip(values.iter())
        .map(|(k, v)| KVQPair { key: k, value: v })
        .collect();
    store.set_many_ref(&items).await?;
    
    // Query for odd numbers, which don't exist, so should return the previous even number
    let query_keys: Vec<Vec<u8>> = (0..50).map(|i| {
        format!("key{:06}", i * 2 + 1).into_bytes()  // Query odd numbers: 1, 3, 5, ..., 99
    }).collect();
    
    let results = store.get_many_leq(&query_keys, 6).await?;  // fuzzy_bytes=6 for the number part
    assert_eq!(results.len(), 50);
    
    for (i, result) in results.iter().enumerate() {
        // We query for odd number (i*2+1), should get the previous even number (i*2)
        assert!(result.is_some(), "Result at index {} should not be None", i);
        assert_eq!(result.as_ref().unwrap(), &values[i], 
            "Failed at index {}: expected value{:03}", i, i);
    }
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_clustering_store_get_many_leq_chunked() -> Result<()> {
    let config = TestConfig::new().await?;
    let store = ScyllaClusteringStore::new(&config.uri, &config.keyspace, "test_clustering_leq", 8).await?;
    
    // Store keys with even clustering keys within each partition
    let keys: Vec<Vec<u8>> = (0..32u64).map(|i| {
        let partition_id = i / 8;
        let clustering_value = (i % 8) * 2;  // Even values: 0, 2, 4, 6, 8, 10, 12, 14
        let mut key = format!("partition{:02}", partition_id).into_bytes();
        key.extend_from_slice(&clustering_value.to_be_bytes());
        key
    }).collect();
    let values: Vec<Vec<u8>> = (0..32).map(|i| format!("value{:03}", i).into_bytes()).collect();
    
    let items: Vec<_> = keys.iter().zip(values.iter())
        .map(|(k, v)| KVQPair { key: k, value: v })
        .collect();
    store.set_many_ref(&items).await?;
    
    // Query for odd clustering keys
    let query_keys: Vec<Vec<u8>> = (0..32u64).map(|i| {
        let partition_id = i / 8;
        let clustering_value = (i % 8) * 2 + 1;  // Odd values: 1, 3, 5, 7, 9, 11, 13, 15
        let mut key = format!("partition{:02}", partition_id).into_bytes();
        key.extend_from_slice(&clustering_value.to_be_bytes());
        key
    }).collect();
    
    let results = store.get_many_leq(&query_keys, 8).await?;
    assert_eq!(results.len(), 32);
    
    for (i, result) in results.iter().enumerate() {
        // We query for odd clustering key, should get the previous even one
        assert!(result.is_some(), "Result at index {} should not be None", i);
        assert_eq!(result.as_ref().unwrap(), &values[i], 
            "Failed at index {}: expected value{:03}", i, i);
    }
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_empty_batch_operations() -> Result<()> {
    let config = TestConfig::new().await?;
    let store = ScyllaKVQStore::new(&config.uri, &config.keyspace, "test_empty_batch").await?;
    
    let empty_keys: Vec<Vec<u8>> = vec![];
    let empty_values: Vec<Vec<u8>> = vec![];
    
    store.set_many_split_ref(&empty_keys, &empty_values).await?;
    let results = store.get_many_exact(&empty_keys).await?;
    assert_eq!(results.len(), 0);
    
    let delete_results = store.delete_many(&empty_keys).await?;
    assert_eq!(delete_results.len(), 0);
    
    let leq_results = store.get_many_leq(&empty_keys, 4).await?;
    assert_eq!(leq_results.len(), 0);
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_batch_stress() -> Result<()> {
    let config = TestConfig::new().await?;
    let store = Arc::new(ScyllaKVQStore::new(&config.uri, &config.keyspace, "test_concurrent_stress").await?);
    
    let mut handles = vec![];
    
    for thread_id in 0..10 {
        let store_clone = store.clone();
        let handle = tokio::spawn(async move {
            let keys: Vec<Vec<u8>> = (0..30).map(|i| {
                format!("thread{:02}key{:03}", thread_id, i).into_bytes()
            }).collect();
            let values: Vec<Vec<u8>> = (0..30).map(|i| {
                format!("thread{:02}value{:03}", thread_id, i).into_bytes()
            }).collect();
            
            let items: Vec<_> = keys.iter().zip(values.iter())
                .map(|(k, v)| KVQPair { key: k, value: v })
                .collect();
            store_clone.set_many_ref(&items).await?;
            
            let results = store_clone.get_many_exact(&keys).await?;
            assert_eq!(results.len(), 30);
            for (i, result) in results.iter().enumerate() {
                assert_eq!(result, &values[i]);
            }
            
            Ok::<_, anyhow::Error>(())
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await??;
    }
    
    config.cleanup().await?;
    Ok(())
}

use std::sync::Arc;