use std::time::Instant;

use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};
use psy_store::store::{scylla::kvq_store::ScyllaKVQStore, KVQlibmdbxStore};
use tempfile::TempDir;

mod common;
use common::*;

async fn benchmark_insert(name: &str, count: usize, insert_fn: impl Fn(usize) -> Result<()>) -> Result<f64> {
    let start = Instant::now();

    for i in 0..count {
        insert_fn(i)?;
    }

    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "{} - Insert {} items: {:.3}s ({:.0} ops/sec)",
        name,
        count,
        elapsed,
        count as f64 / elapsed
    );
    Ok(elapsed)
}

async fn benchmark_read(name: &str, count: usize, read_fn: impl Fn(usize) -> Result<Vec<u8>>) -> Result<f64> {
    let start = Instant::now();

    for i in 0..count {
        let _ = read_fn(i)?;
    }

    let elapsed = start.elapsed().as_secs_f64();
    println!("{} - Read {} items: {:.3}s ({:.0} ops/sec)", name, count, elapsed, count as f64 / elapsed);
    Ok(elapsed)
}

async fn benchmark_batch_ops<S: KVQBinaryStoreAsync>(name: &str, store: &S, count: usize) -> Result<()> {
    let batch_sizes = vec![1, 15, 30, 100];

    for batch_size in batch_sizes {
        let num_batches = count / batch_size;
        let keys: Vec<Vec<u8>> = (0..count).map(|i| format!("key{:08}", i).into_bytes()).collect();
        let values: Vec<Vec<u8>> = (0..count).map(|i| format!("value{:08}", i).into_bytes()).collect();

        let start = Instant::now();
        for batch_idx in 0..num_batches {
            let start_idx = batch_idx * batch_size;
            let end_idx = std::cmp::min(start_idx + batch_size, count);
            let batch_items: Vec<_> = (start_idx..end_idx)
                .map(|i| KVQPair {
                    key: &keys[i],
                    value: &values[i],
                })
                .collect();
            store.set_many_ref(&batch_items).await?;
        }
        let insert_elapsed = start.elapsed().as_secs_f64();
        println!(
            "{} - Batch insert (size {}): {:.3}s ({:.0} ops/sec)",
            name,
            batch_size,
            insert_elapsed,
            count as f64 / insert_elapsed
        );

        let start = Instant::now();
        for batch_idx in 0..num_batches {
            let start_idx = batch_idx * batch_size;
            let end_idx = std::cmp::min(start_idx + batch_size, count);
            let batch_keys: Vec<_> = (start_idx..end_idx).map(|i| keys[i].clone()).collect();
            let _ = store.get_many_exact(&batch_keys).await?;
        }
        let read_elapsed = start.elapsed().as_secs_f64();
        println!(
            "{} - Batch read (size {}): {:.3}s ({:.0} ops/sec)",
            name,
            batch_size,
            read_elapsed,
            count as f64 / read_elapsed
        );

        store.delete_many(&keys).await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_performance_small_dataset() -> Result<()> {
    println!("\n=== Small Dataset Test (1,000 items) ===");
    let count = 1_000;

    let scylla_config = TestConfig::new().await?;
    let scylla_store = ScyllaKVQStore::new(&scylla_config.uri, &scylla_config.keyspace, "perf_small").await?;

    let mdbx_dir = TempDir::new()?;
    let mdbx_store = KVQlibmdbxStore::new_write(mdbx_dir.path().to_str().unwrap())?;

    let keys: Vec<Vec<u8>> = (0..count).map(|i| format!("key{:08}", i).into_bytes()).collect();
    let values: Vec<Vec<u8>> = (0..count).map(|i| format!("value{:08}", i).into_bytes()).collect();

    println!("\n--- ScyllaDB Single Operations ---");
    let start = Instant::now();
    for i in 0..count {
        scylla_store.set(keys[i].clone(), values[i].clone()).await?;
    }
    let scylla_insert = start.elapsed().as_secs_f64();
    println!("Insert: {:.3}s ({:.0} ops/sec)", scylla_insert, count as f64 / scylla_insert);

    let start = Instant::now();
    for i in 0..count {
        let _ = scylla_store.get_exact(&keys[i]).await?;
    }
    let scylla_read = start.elapsed().as_secs_f64();
    println!("Read: {:.3}s ({:.0} ops/sec)", scylla_read, count as f64 / scylla_read);

    println!("\n--- MDBX Single Operations ---");
    let start = Instant::now();
    for i in 0..count {
        <KVQlibmdbxStore as KVQBinaryStore>::set_ref(&mdbx_store, &keys[i], &values[i])?;
    }
    let mdbx_insert = start.elapsed().as_secs_f64();
    println!("Insert: {:.3}s ({:.0} ops/sec)", mdbx_insert, count as f64 / mdbx_insert);

    let start = Instant::now();
    for i in 0..count {
        let _ = <KVQlibmdbxStore as KVQBinaryStore>::get_exact(&mdbx_store, &keys[i])?;
    }
    let mdbx_read = start.elapsed().as_secs_f64();
    println!("Read: {:.3}s ({:.0} ops/sec)", mdbx_read, count as f64 / mdbx_read);

    println!("\n--- Batch Operations ---");
    scylla_store.delete_many(&keys).await?;
    benchmark_batch_ops("ScyllaDB", &scylla_store, count).await?;

    scylla_config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_performance_medium_dataset() -> Result<()> {
    println!("\n=== Medium Dataset Test (10,000 items) ===");
    let count = 10_000;

    let scylla_config = TestConfig::new().await?;
    let scylla_store = ScyllaKVQStore::new(&scylla_config.uri, &scylla_config.keyspace, "perf_medium").await?;

    let keys: Vec<Vec<u8>> = (0..count).map(|i| format!("key{:08}", i).into_bytes()).collect();
    let values: Vec<Vec<u8>> = (0..count).map(|i| format!("value{:08}", i).into_bytes()).collect();

    println!("\n--- ScyllaDB Batch Operations ---");
    benchmark_batch_ops("ScyllaDB", &scylla_store, count).await?;

    scylla_config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_performance_fuzzy_search() -> Result<()> {
    println!("\n=== Fuzzy Search Performance Test ===");
    let count = 5_000;

    let scylla_config = TestConfig::new().await?;
    let scylla_store = ScyllaKVQStore::new(&scylla_config.uri, &scylla_config.keyspace, "perf_fuzzy").await?;

    let mdbx_dir = TempDir::new()?;
    let mdbx_store = KVQlibmdbxStore::new_write(mdbx_dir.path().to_str().unwrap())?;

    let keys: Vec<Vec<u8>> = (0..count).map(|i| format!("prefix{:04}suffix{:04}", i / 100, i).into_bytes()).collect();
    let values: Vec<Vec<u8>> = (0..count).map(|i| format!("value{:08}", i).into_bytes()).collect();

    let items: Vec<_> = keys.iter().zip(values.iter()).map(|(k, v)| KVQPair { key: k, value: v }).collect();
    scylla_store.set_many_ref(&items).await?;

    for i in 0..count {
        <KVQlibmdbxStore as KVQBinaryStore>::set_ref(&mdbx_store, &keys[i], &values[i])?;
    }

    let query_keys: Vec<Vec<u8>> = (0..1000)
        .map(|i| format!("prefix{:04}suffix{:04}", i / 100, i * 5 + 2).into_bytes())
        .collect();

    println!("\n--- ScyllaDB Fuzzy Search ---");
    let start = Instant::now();
    let _ = scylla_store.get_many_leq(&query_keys, 4).await?;
    let scylla_fuzzy = start.elapsed().as_secs_f64();
    println!("get_many_leq (1000 queries): {:.3}s ({:.0} ops/sec)", scylla_fuzzy, 1000.0 / scylla_fuzzy);

    println!("\n--- MDBX Fuzzy Search ---");
    let start = Instant::now();
    for key in &query_keys {
        let _ = <KVQlibmdbxStore as KVQBinaryStore>::get_leq(&mdbx_store, key, 4)?;
    }
    let mdbx_fuzzy = start.elapsed().as_secs_f64();
    println!("get_leq (1000 queries): {:.3}s ({:.0} ops/sec)", mdbx_fuzzy, 1000.0 / mdbx_fuzzy);

    scylla_config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_performance_concurrent_access() -> Result<()> {
    println!("\n=== Concurrent Access Performance Test ===");

    let scylla_config = TestConfig::new().await?;
    let scylla_store = Arc::new(ScyllaKVQStore::new(&scylla_config.uri, &scylla_config.keyspace, "perf_concurrent").await?);

    let thread_count = 10;
    let ops_per_thread = 1000;

    println!("\n--- ScyllaDB Concurrent Operations ---");
    let start = Instant::now();
    let mut handles = vec![];

    for thread_id in 0..thread_count {
        let store_clone = scylla_store.clone();
        let handle = tokio::spawn(async move {
            let keys: Vec<Vec<u8>> = (0..ops_per_thread)
                .map(|i| format!("thread{:02}key{:05}", thread_id, i).into_bytes())
                .collect();
            let values: Vec<Vec<u8>> = (0..ops_per_thread).map(|i| format!("value{:08}", i).into_bytes()).collect();

            let items: Vec<_> = keys.iter().zip(values.iter()).map(|(k, v)| KVQPair { key: k, value: v }).collect();

            for chunk in items.chunks(15) {
                store_clone.set_many_ref(chunk).await?;
            }

            for chunk in keys.chunks(15) {
                let _ = store_clone.get_many_exact(chunk).await?;
            }

            Ok::<_, anyhow::Error>(())
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await??;
    }

    let elapsed = start.elapsed().as_secs_f64();
    let total_ops = thread_count * ops_per_thread * 2;
    println!("Concurrent operations: {:.3}s ({:.0} ops/sec)", elapsed, total_ops as f64 / elapsed);

    scylla_config.cleanup().await?;
    Ok(())
}

use std::sync::Arc;
