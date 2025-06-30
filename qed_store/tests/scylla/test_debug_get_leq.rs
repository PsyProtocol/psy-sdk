use kvq::traits::{KVQBinaryStoreAsync, KVQPair};
use qed_store::store::scylla::kvq_store::ScyllaKVQStore;
use anyhow::Result;

mod common;
use common::*;

#[tokio::test]
async fn test_simple_get_leq() -> Result<()> {
    let config = TestConfig::new().await?;
    let store = ScyllaKVQStore::new(&config.uri, &config.keyspace, "test_simple_leq").await?;
    
    // Store a simple key
    let key = b"prefix00suffix0000".to_vec();
    let value = b"value000".to_vec();
    store.set(key.clone(), value.clone()).await?;
    
    // Query for a key that doesn't exist but is larger
    let query_key = b"prefix00suffix0001".to_vec();
    
    // fuzzy_bytes = 4 means we're looking at the last 4 bytes (0001)
    // The prefix "prefix00suffix" should match, and we want the largest key <= query_key
    let result = store.get_leq(&query_key, 4).await?;
    
    println!("Stored key: {:?}", String::from_utf8_lossy(&key));
    println!("Query key: {:?}", String::from_utf8_lossy(&query_key));
    println!("Result: {:?}", result.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    assert!(result.is_some(), "get_leq should return a result");
    assert_eq!(result.unwrap(), value, "Should return the value for the stored key");
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_get_leq_with_exact_match() -> Result<()> {
    let config = TestConfig::new().await?;
    let store = ScyllaKVQStore::new(&config.uri, &config.keyspace, "test_exact_leq").await?;
    
    // Store multiple keys
    let keys = vec![
        b"prefix00suffix0000".to_vec(),
        b"prefix00suffix0001".to_vec(),
        b"prefix00suffix0002".to_vec(),
    ];
    let values = vec![
        b"value000".to_vec(),
        b"value001".to_vec(),
        b"value002".to_vec(),
    ];
    
    for (k, v) in keys.iter().zip(values.iter()) {
        store.set(k.clone(), v.clone()).await?;
    }
    
    // Query for an exact match
    let result = store.get_leq(&keys[1], 4).await?;
    assert_eq!(result, Some(values[1].clone()));
    
    // Query for a key between existing keys
    let query_key = b"prefix00suffix0003".to_vec();
    let result = store.get_leq(&query_key, 4).await?;
    println!("Query for {:?}, got {:?}", 
        String::from_utf8_lossy(&query_key), 
        result.as_ref().map(|v| String::from_utf8_lossy(v)));
    assert_eq!(result, Some(values[2].clone()), "Should return value002");
    
    config.cleanup().await?;
    Ok(())
}

#[tokio::test] 
async fn test_get_leq_no_match() -> Result<()> {
    let config = TestConfig::new().await?;
    let store = ScyllaKVQStore::new(&config.uri, &config.keyspace, "test_no_match_leq").await?;
    
    // Store a key with different prefix
    let key = b"prefix01suffix0000".to_vec();
    let value = b"value100".to_vec();
    store.set(key.clone(), value.clone()).await?;
    
    // Query for a key with different prefix
    let query_key = b"prefix00suffix0001".to_vec();
    let result = store.get_leq(&query_key, 4).await?;
    
    println!("Stored: {:?}", String::from_utf8_lossy(&key));
    println!("Query: {:?}", String::from_utf8_lossy(&query_key));
    println!("Result: {:?}", result.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    // With fuzzy_bytes=4, we're looking for keys with prefix "prefix00suffix"
    // Since we only have "prefix01suffix0000", there should be no match
    assert!(result.is_none(), "Should return None when no matching prefix");
    
    config.cleanup().await?;
    Ok(())
}