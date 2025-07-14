use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQPair};
use qed_store::store::lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use qed_data::config::store_config::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_kvq_store_fuzzy_operations() -> Result<()> {
    println!("=== Testing KVQ Store Fuzzy Operations ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_kvq_fuzzy");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    println!("✓ LibMDBX initialized");
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_kvq_fuzzy").await {
        Ok(store) => {
            println!("✓ ScyllaDB initialized");
            store
        },
        Err(e) => {
            println!("✗ ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    // Test with a table that uses KVQ store (not clustering)
    let table_type = USER_PUBLIC_KEY_HELPER_TABLE_TYPE;
    
    println!("\n1. Testing get_leq with KVQ store");
    
    // Insert test data with similar structure
    let base_prefix = vec![0x00, 0x11]; // table type
    let user_id = vec![0x01, 0x02, 0x03, 0x04]; // user identifier
    
    for version in &[1u32, 5, 10, 15, 20] {
        let mut key = base_prefix.clone();
        key.extend_from_slice(&user_id);
        key.extend_from_slice(&version.to_be_bytes());
        
        let value = format!("user_key_v{}", version).into_bytes();
        
        mdbx_store.set_ref(&key, &value)?;
        <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla_store, &key, &value).await?;
        
        println!("   Inserted version {} - key: {:?}", version, key);
    }
    
    // Test exact match
    println!("\n2. Testing exact match (fuzzy_bytes=0)");
    let mut test_key = base_prefix.clone();
    test_key.extend_from_slice(&user_id);
    test_key.extend_from_slice(&10u32.to_be_bytes());
    
    let mdbx_exact = mdbx_store.get_leq(&test_key, 0)?;
    let scylla_exact = <ScyllaStore as KVQBinaryStoreAsync>::get_leq(&scylla_store, &test_key, 0).await?;
    
    println!("   LibMDBX: {:?}", mdbx_exact.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   ScyllaDB: {:?}", scylla_exact.as_ref().map(|v| String::from_utf8_lossy(v)));
    assert_eq!(mdbx_exact, scylla_exact);
    
    // Test fuzzy match
    println!("\n3. Testing fuzzy match (fuzzy_bytes=4)");
    for query_version in &[3u32, 7, 12, 18, 25] {
        let mut query_key = base_prefix.clone();
        query_key.extend_from_slice(&user_id);
        query_key.extend_from_slice(&query_version.to_be_bytes());
        
        let mdbx_fuzzy = mdbx_store.get_leq(&query_key, 4)?;
        let scylla_fuzzy = <ScyllaStore as KVQBinaryStoreAsync>::get_leq(&scylla_store, &query_key, 4).await?;
        
        println!("   Query version {} - LibMDBX: {:?}, ScyllaDB: {:?}", 
                 query_version,
                 mdbx_fuzzy.as_ref().map(|v| String::from_utf8_lossy(v)),
                 scylla_fuzzy.as_ref().map(|v| String::from_utf8_lossy(v)));
        
        assert_eq!(mdbx_fuzzy, scylla_fuzzy, "Fuzzy match should be consistent");
    }
    
    println!("\n4. Testing get_fuzzy_range_leq_kv");
    
    // Query for all versions <= 12
    let mut range_key = base_prefix.clone();
    range_key.extend_from_slice(&user_id);
    range_key.extend_from_slice(&12u32.to_be_bytes());
    
    let mdbx_range = mdbx_store.get_fuzzy_range_leq_kv(&range_key, 4)?;
    let scylla_range = <ScyllaStore as KVQBinaryStoreAsync>::get_fuzzy_range_leq_kv(&scylla_store, &range_key, 4).await?;
    
    println!("   Query: versions <= 12");
    println!("   LibMDBX found: {} entries", mdbx_range.len());
    println!("   ScyllaDB found: {} entries", scylla_range.len());
    
    // Both should find versions 1, 5, and 10
    assert_eq!(mdbx_range.len(), scylla_range.len(), 
               "Should find same number of entries");
    
    for (i, (mdbx_kv, scylla_kv)) in mdbx_range.iter().zip(scylla_range.iter()).enumerate() {
        println!("   Entry {} - LibMDBX: {:?}, ScyllaDB: {:?}",
                 i,
                 String::from_utf8_lossy(&mdbx_kv.value),
                 String::from_utf8_lossy(&scylla_kv.value));
        assert_eq!(mdbx_kv.value, scylla_kv.value);
    }
    
    println!("\n✅ KVQ store fuzzy operations are now consistent!");
    
    Ok(())
}