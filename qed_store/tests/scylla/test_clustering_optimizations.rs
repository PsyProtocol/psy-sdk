use kvq::traits::{KVQBinaryStoreAsync, KVQPair};
use qed_store::store::scylla::clustering_store::ScyllaClusteringStore;
use std::time::Instant;
use anyhow::Result;

mod common;
use common::*;

#[tokio::test]
async fn test_clustering_get_leq_with_clustering_key_size() -> Result<()> {
    let config = TestConfig::new().await?;
    let clustering_key_size = 8;
    let store = ScyllaClusteringStore::new(
        &config.uri, 
        &config.keyspace, 
        "test_clustering_leq_opt", 
        clustering_key_size
    ).await?;
    
    let partition_key = b"partition001";
    let clustering_keys: Vec<Vec<u8>> = (0..10u64).map(|i| i.to_be_bytes().to_vec()).collect();
    
    let mut keys = Vec::new();
    let mut values = Vec::new();
    for (i, ck) in clustering_keys.iter().enumerate() {
        let mut key = partition_key.to_vec();
        key.extend_from_slice(ck);
        keys.push(key);
        values.push(format!("value{:02}", i).into_bytes());
    }
    
    let items: Vec<_> = keys.iter().zip(values.iter())
        .map(|(k, v)| KVQPair { key: k, value: v })
        .collect();
    store.set_many_ref(&items).await?;
    
    let mut query_key = partition_key.to_vec();
    query_key.extend_from_slice(&7u64.to_be_bytes());
    
    let result = store.get_leq(&query_key, clustering_key_size).await?;
    assert!(result.is_some());
    assert_eq!(result.unwrap(), b"value07");
    
    let mut query_key_between = partition_key.to_vec();
    query_key_between.extend_from_slice(&15u64.to_be_bytes());
    
    let result = store.get_leq(&query_key_between, clustering_key_size).await?;
    assert!(result.is_some());
    assert_eq!(result.unwrap(), b"value09");
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_clustering_get_fuzzy_range_performance() -> Result<()> {
    let config = TestConfig::new().await?;
    let clustering_key_size = 8;
    let store = ScyllaClusteringStore::new(
        &config.uri, 
        &config.keyspace, 
        "test_clustering_range_perf", 
        clustering_key_size
    ).await?;
    
    let partitions = 10;
    let items_per_partition = 100;
    
    let mut all_items = Vec::new();
    for p in 0..partitions {
        let partition_key = format!("partition{:03}", p).into_bytes();
        for i in 0..items_per_partition {
            let mut key = partition_key.clone();
            key.extend_from_slice(&(i as u64).to_be_bytes());
            let value = format!("p{}i{}", p, i).into_bytes();
            all_items.push(KVQPair { key, value });
        }
    }
    
    let refs: Vec<_> = all_items.iter()
        .map(|item| KVQPair { key: &item.key, value: &item.value })
        .collect();
    store.set_many_ref(&refs).await?;
    
    println!("\n--- Testing get_fuzzy_range_leq_kv performance ---");
    
    let mut query_key = b"partition005".to_vec();
    query_key.extend_from_slice(&50u64.to_be_bytes());
    
    let start = Instant::now();
    let results = store.get_fuzzy_range_leq_kv(&query_key, clustering_key_size).await?;
    let elapsed = start.elapsed().as_secs_f64();
    
    assert_eq!(results.len(), 51);
    println!("Query single partition (51 items): {:.4}s", elapsed);
    
    let start = Instant::now();
    let results = store.get_fuzzy_range_leq_kv(&query_key, 4).await?;
    let elapsed = start.elapsed().as_secs_f64();
    
    assert!(results.len() > 0);
    println!("Query with fuzzy prefix (cross-partition): {:.4}s", elapsed);
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_clustering_batch_operations_performance() -> Result<()> {
    let config = TestConfig::new().await?;
    let clustering_key_size = 8;
    let store = ScyllaClusteringStore::new(
        &config.uri, 
        &config.keyspace, 
        "test_clustering_batch_perf", 
        clustering_key_size
    ).await?;
    
    let test_sizes = vec![15, 30, 45, 75, 150];
    
    for size in test_sizes {
        println!("\n--- Testing batch size: {} ---", size);
        
        let mut keys = Vec::new();
        let mut values = Vec::new();
        
        for i in 0..size {
            let partition = format!("part{:03}", i / 15);
            let mut key = partition.into_bytes();
            key.extend_from_slice(&(i as u64).to_be_bytes());
            keys.push(key);
            values.push(format!("value{:05}", i).into_bytes());
        }
        
        let items: Vec<_> = keys.iter().zip(values.iter())
            .map(|(k, v)| KVQPair { key: k, value: v })
            .collect();
        
        let start = Instant::now();
        store.set_many_ref(&items).await?;
        let insert_time = start.elapsed().as_secs_f64();
        println!("Batch insert: {:.4}s ({:.0} ops/sec)", insert_time, size as f64 / insert_time);
        
        let start = Instant::now();
        let results = store.get_many_exact(&keys).await?;
        let read_time = start.elapsed().as_secs_f64();
        assert_eq!(results.len(), size);
        println!("Batch read: {:.4}s ({:.0} ops/sec)", read_time, size as f64 / read_time);
        
        let start = Instant::now();
        let delete_results = store.delete_many(&keys).await?;
        let delete_time = start.elapsed().as_secs_f64();
        assert_eq!(delete_results.len(), size);
        println!("Batch delete: {:.4}s ({:.0} ops/sec)", delete_time, size as f64 / delete_time);
    }
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_clustering_concurrent_batch_operations() -> Result<()> {
    let config = TestConfig::new().await?;
    let clustering_key_size = 8;
    let store = Arc::new(ScyllaClusteringStore::new(
        &config.uri, 
        &config.keyspace, 
        "test_clustering_concurrent", 
        clustering_key_size
    ).await?);
    
    let num_threads = 20;
    let items_per_thread = 150;
    
    println!("\n--- Testing concurrent batch operations ---");
    let start = Instant::now();
    
    let mut handles = vec![];
    for thread_id in 0..num_threads {
        let store_clone = store.clone();
        let handle = tokio::spawn(async move {
            let mut keys = Vec::new();
            let mut values = Vec::new();
            
            for i in 0..items_per_thread {
                let partition = format!("thread{:02}part{:02}", thread_id, i / 50);
                let mut key = partition.into_bytes();
                key.extend_from_slice(&(i as u64).to_be_bytes());
                keys.push(key);
                values.push(format!("t{}v{}", thread_id, i).into_bytes());
            }
            
            let items: Vec<_> = keys.iter().zip(values.iter())
                .map(|(k, v)| KVQPair { key: k, value: v })
                .collect();
            
            store_clone.set_many_ref(&items).await?;
            
            let results = store_clone.get_many_exact(&keys).await?;
            assert_eq!(results.len(), items_per_thread);
            
            Ok::<_, anyhow::Error>(())
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await??;
    }
    
    let elapsed = start.elapsed().as_secs_f64();
    let total_ops = num_threads * items_per_thread * 2;
    println!("Total time: {:.3}s ({:.0} ops/sec)", elapsed, total_ops as f64 / elapsed);
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_clustering_get_many_leq_chunked() -> Result<()> {
    let config = TestConfig::new().await?;
    let clustering_key_size = 8;
    let store = ScyllaClusteringStore::new(
        &config.uri, 
        &config.keyspace, 
        "test_clustering_many_leq", 
        clustering_key_size
    ).await?;
    
    let mut all_keys = Vec::new();
    let mut all_values = Vec::new();
    
    for p in 0..5 {
        let partition = format!("partition{:02}", p).into_bytes();
        for i in 0..20u64 {
            let mut key = partition.clone();
            key.extend_from_slice(&(i * 2).to_be_bytes());
            all_keys.push(key);
            all_values.push(format!("p{}i{}", p, i).into_bytes());
        }
    }
    
    let items: Vec<_> = all_keys.iter().zip(all_values.iter())
        .map(|(k, v)| KVQPair { key: k, value: v })
        .collect();
    store.set_many_ref(&items).await?;
    
    let mut query_keys = Vec::new();
    for p in 0..5 {
        let partition = format!("partition{:02}", p).into_bytes();
        for i in 0..20u64 {
            let mut key = partition.clone();
            key.extend_from_slice(&(i * 2 + 1).to_be_bytes());
            query_keys.push(key);
        }
    }
    
    let start = Instant::now();
    let results = store.get_many_leq(&query_keys, clustering_key_size).await?;
    let elapsed = start.elapsed().as_secs_f64();
    
    assert_eq!(results.len(), 100);
    // All queries should return results because we're querying for odd values
    // and we have even values stored, so each query finds the previous even value
    let non_none_count = results.iter().filter(|r| r.is_some()).count();
    assert_eq!(non_none_count, 100);
    
    println!("get_many_leq for 100 keys: {:.4}s ({:.0} ops/sec)", elapsed, 100.0 / elapsed);
    
    config.cleanup().await?;
    Ok(())
}

use std::sync::Arc;