use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};
use qed_store::store::scylla::kvq_store::ScyllaKVQStore;
use qed_store::store::lmdbx::KVQlibmdbxStore;
use std::time::Instant;
use anyhow::Result;
use tempfile::TempDir;

mod common;
use common::*;

#[tokio::test]
async fn test_performance_comparison_detailed() -> Result<()> {
    println!("\n=== Detailed Performance Comparison: ScyllaDB vs MDBX ===\n");
    
    let scylla_config = TestConfig::new().await?;
    let scylla_store = ScyllaKVQStore::new(&scylla_config.uri, &scylla_config.keyspace, "perf_detailed").await?;
    
    let mdbx_dir = TempDir::new()?;
    let mdbx_store = KVQlibmdbxStore::new_write(mdbx_dir.path().to_str().unwrap())?;
    
    // Test different data sizes
    let test_sizes = vec![100, 1000, 10000];
    
    for size in test_sizes {
        println!("\n--- Dataset Size: {} items ---", size);
        
        let keys: Vec<Vec<u8>> = (0..size).map(|i| format!("key{:08}", i).into_bytes()).collect();
        let values: Vec<Vec<u8>> = (0..size).map(|i| format!("value{:08}", i).into_bytes()).collect();
        
        // ScyllaDB batch insert
        let start = Instant::now();
        let items: Vec<_> = keys.iter().zip(values.iter())
            .map(|(k, v)| KVQPair { key: k, value: v })
            .collect();
        scylla_store.set_many_ref(&items).await?;
        let scylla_batch_insert = start.elapsed().as_secs_f64();
        println!("ScyllaDB Batch Insert: {:.3}s ({:.0} ops/sec)", 
            scylla_batch_insert, size as f64 / scylla_batch_insert);
        
        // MDBX individual inserts (no batch API)
        let start = Instant::now();
        for i in 0..size {
            mdbx_store.set(keys[i].clone(), values[i].clone())?;
        }
        let mdbx_insert = start.elapsed().as_secs_f64();
        println!("MDBX Insert: {:.3}s ({:.0} ops/sec)", 
            mdbx_insert, size as f64 / mdbx_insert);
        
        // ScyllaDB batch read
        let start = Instant::now();
        let _ = scylla_store.get_many_exact(&keys).await?;
        let scylla_batch_read = start.elapsed().as_secs_f64();
        println!("ScyllaDB Batch Read: {:.3}s ({:.0} ops/sec)", 
            scylla_batch_read, size as f64 / scylla_batch_read);
        
        // MDBX individual reads
        let start = Instant::now();
        for key in &keys {
            let _ = mdbx_store.get_exact(key)?;
        }
        let mdbx_read = start.elapsed().as_secs_f64();
        println!("MDBX Read: {:.3}s ({:.0} ops/sec)", 
            mdbx_read, size as f64 / mdbx_read);
        
        // Performance ratios
        println!("\nPerformance Comparison:");
        println!("- Insert: ScyllaDB is {:.1}x {} than MDBX", 
            if scylla_batch_insert < mdbx_insert { mdbx_insert / scylla_batch_insert } else { scylla_batch_insert / mdbx_insert },
            if scylla_batch_insert < mdbx_insert { "faster" } else { "slower" });
        println!("- Read: ScyllaDB is {:.1}x {} than MDBX", 
            if scylla_batch_read < mdbx_read { mdbx_read / scylla_batch_read } else { scylla_batch_read / mdbx_read },
            if scylla_batch_read < mdbx_read { "faster" } else { "slower" });
        
        // Clean up ScyllaDB data for next test
        scylla_store.delete_many(&keys).await?;
    }
    
    scylla_config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_get_leq_performance_comparison() -> Result<()> {
    println!("\n=== get_leq Performance Comparison ===\n");
    
    let scylla_config = TestConfig::new().await?;
    let scylla_store = ScyllaKVQStore::new(&scylla_config.uri, &scylla_config.keyspace, "perf_leq").await?;
    
    let mdbx_dir = TempDir::new()?;
    let mdbx_store = KVQlibmdbxStore::new_write(mdbx_dir.path().to_str().unwrap())?;
    
    // Insert test data
    let count = 10000;
    let keys: Vec<Vec<u8>> = (0..count).step_by(2).map(|i| {
        format!("key{:08}", i).into_bytes()
    }).collect();
    let values: Vec<Vec<u8>> = (0..count/2).map(|i| {
        format!("value{:08}", i).into_bytes()
    }).collect();
    
    // Insert into both stores
    let items: Vec<_> = keys.iter().zip(values.iter())
        .map(|(k, v)| KVQPair { key: k, value: v })
        .collect();
    scylla_store.set_many_ref(&items).await?;
    
    for (k, v) in keys.iter().zip(values.iter()) {
        mdbx_store.set(k.clone(), v.clone())?;
    }
    
    // Test get_leq performance
    let query_count = 1000;
    let query_keys: Vec<Vec<u8>> = (0..query_count).map(|i| {
        format!("key{:08}", i * 10 + 1).into_bytes() // Query for non-existent odd keys
    }).collect();
    
    // ScyllaDB get_many_leq
    let start = Instant::now();
    let _ = scylla_store.get_many_leq(&query_keys, 4).await?;
    let scylla_time = start.elapsed().as_secs_f64();
    println!("ScyllaDB get_many_leq: {:.3}s ({:.0} ops/sec)", 
        scylla_time, query_count as f64 / scylla_time);
    
    // MDBX individual get_leq
    let start = Instant::now();
    for key in &query_keys {
        let _ = mdbx_store.get_leq(key, 4)?;
    }
    let mdbx_time = start.elapsed().as_secs_f64();
    println!("MDBX get_leq: {:.3}s ({:.0} ops/sec)", 
        mdbx_time, query_count as f64 / mdbx_time);
    
    println!("\nget_leq Performance: ScyllaDB is {:.1}x {} than MDBX", 
        if scylla_time < mdbx_time { mdbx_time / scylla_time } else { scylla_time / mdbx_time },
        if scylla_time < mdbx_time { "faster" } else { "slower" });
    
    scylla_config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_performance() -> Result<()> {
    println!("\n=== Concurrent Operations Performance ===\n");
    
    let scylla_config = TestConfig::new().await?;
    let scylla_store = Arc::new(ScyllaKVQStore::new(&scylla_config.uri, &scylla_config.keyspace, "perf_concurrent").await?);
    
    let thread_counts = vec![1, 5, 10, 20];
    let ops_per_thread = 500;
    
    for thread_count in thread_counts {
        println!("\n--- {} Concurrent Threads ---", thread_count);
        
        let start = Instant::now();
        let mut handles = vec![];
        
        for thread_id in 0..thread_count {
            let store_clone = scylla_store.clone();
            let handle = tokio::spawn(async move {
                let keys: Vec<Vec<u8>> = (0..ops_per_thread).map(|i| {
                    format!("t{:02}k{:05}", thread_id, i).into_bytes()
                }).collect();
                let values: Vec<Vec<u8>> = (0..ops_per_thread).map(|i| {
                    format!("value{:08}", i).into_bytes()
                }).collect();
                
                // Batch operations
                let items: Vec<_> = keys.iter().zip(values.iter())
                    .map(|(k, v)| KVQPair { key: k, value: v })
                    .collect();
                
                store_clone.set_many_ref(&items).await?;
                let _ = store_clone.get_many_exact(&keys).await?;
                
                Ok::<_, anyhow::Error>(())
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await??;
        }
        
        let elapsed = start.elapsed().as_secs_f64();
        let total_ops = thread_count * ops_per_thread * 2; // Both read and write
        println!("ScyllaDB: {:.3}s ({:.0} ops/sec)", elapsed, total_ops as f64 / elapsed);
        
        // Note: MDBX doesn't support concurrent writes from multiple threads
        println!("MDBX: Not tested (single-writer limitation)");
    }
    
    scylla_config.cleanup().await?;
    Ok(())
}

use std::sync::Arc;