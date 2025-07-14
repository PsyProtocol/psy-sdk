use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync};
use qed_store::store::lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use qed_data::config::store_config::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_fuzzy_across_partitions() -> Result<()> {
    println!("=== Testing Fuzzy Matching Across Partition Boundaries ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_fuzzy_across");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_fuzzy_across").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    // For user leaves table:
    // Key: [table_type(2) + user_id(8) + version(4)] = 14 bytes
    // Clustering key size = 4 (version)
    // Partition key size = 10 (table_type + user_id)
    
    let table_type = USER_LEAF_TABLE_TYPE;
    
    println!("1. Setting up test data:");
    println!("   User leaves table - clustering_key_size = 4");
    println!("   Key structure: [table_type(2) + user_id(8) + version(4)] = 14 bytes");
    
    // Insert data for different users
    let users = vec![1000u64, 2000, 3000];
    let versions = vec![1u32, 2, 3];
    
    for user_id in &users {
        for version in &versions {
            let mut key = table_type.to_be_bytes().to_vec();
            key.extend_from_slice(&user_id.to_be_bytes());
            key.extend_from_slice(&version.to_be_bytes());
            
            let value = format!("user_{}_v{}", user_id, version).into_bytes();
            
            mdbx_store.set_ref(&key, &value)?;
            <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla_store, &key, &value).await?;
            
            println!("   Inserted user {} version {} - key: {:?}", user_id, version, key);
        }
    }
    
    println!("\n2. Testing fuzzy_bytes = 6 (spans across partition boundary):");
    println!("   This fuzzy size covers part of user_id and all of version");
    println!("   Requires scanning all partitions, not just one");
    
    // Query key: user 2500, version 5
    let mut query_key = table_type.to_be_bytes().to_vec();
    query_key.extend_from_slice(&2500u64.to_be_bytes());
    query_key.extend_from_slice(&5u32.to_be_bytes());
    
    println!("\n   Query key: user 2500, version 5");
    println!("   Key bytes: {:?}", query_key);
    
    let mdbx_result = mdbx_store.get_leq(&query_key, 6)?;
    let scylla_result = <ScyllaStore as KVQBinaryStoreAsync>::get_leq(&scylla_store, &query_key, 6).await?;
    
    println!("\n   Results with fuzzy_bytes = 6:");
    println!("   - LibMDBX: {:?}", mdbx_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    // With fuzzy_bytes = 6:
    // - Prefix comparison: first 8 bytes (table_type + first 6 bytes of user_id)
    // - Suffix comparison: last 6 bytes (last 2 bytes of user_id + 4 bytes of version)
    // This should find matches across different user partitions!
    
    println!("\n3. Testing fuzzy_bytes = 4 (matches clustering key size):");
    
    let mdbx_result_4 = mdbx_store.get_leq(&query_key, 4)?;
    let scylla_result_4 = <ScyllaStore as KVQBinaryStoreAsync>::get_leq(&scylla_store, &query_key, 4).await?;
    
    println!("   - LibMDBX: {:?}", mdbx_result_4.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_result_4.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    println!("\n4. Testing fuzzy_bytes = 12 (almost entire key):");
    
    let mdbx_result_12 = mdbx_store.get_leq(&query_key, 12)?;
    let scylla_result_12 = <ScyllaStore as KVQBinaryStoreAsync>::get_leq(&scylla_store, &query_key, 12).await?;
    
    println!("   - LibMDBX: {:?}", mdbx_result_12.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_result_12.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    // Verify consistency
    assert_eq!(mdbx_result, scylla_result, "fuzzy_bytes=6 mismatch");
    assert_eq!(mdbx_result_4, scylla_result_4, "fuzzy_bytes=4 mismatch");
    assert_eq!(mdbx_result_12, scylla_result_12, "fuzzy_bytes=12 mismatch");
    
    println!("\n✅ All fuzzy matching tests passed!");
    
    Ok(())
}