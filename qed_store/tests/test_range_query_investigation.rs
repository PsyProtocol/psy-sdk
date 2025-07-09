use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQPair};
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use qed_data::config::store_config::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_range_query_investigation() -> Result<()> {
    println!("=== Investigating Range Query Behavior ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_range_query");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_range_query").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    let table_type = USER_LEAF_TABLE_TYPE;
    
    println!("1. Setting up test data for user 1000:");
    
    // Insert consecutive versions for user 1000
    for version in 1u32..=10 {
        let mut key = table_type.to_be_bytes().to_vec();
        key.extend_from_slice(&1000u64.to_be_bytes());
        key.extend_from_slice(&version.to_be_bytes());
        
        let value = format!("user_1000_v{}", version).into_bytes();
        
        mdbx_store.set_ref(&key, &value)?;
        scylla_store.set_ref(&key, &value)?;
        
        println!("   Inserted version {} - key: {:?}", version, key);
    }
    
    // Also insert data for other users to ensure isolation
    for user_id in [999u64, 1001] {
        let mut key = table_type.to_be_bytes().to_vec();
        key.extend_from_slice(&user_id.to_be_bytes());
        key.extend_from_slice(&1u32.to_be_bytes());
        
        let value = format!("user_{}_v1", user_id).into_bytes();
        
        mdbx_store.set_ref(&key, &value)?;
        scylla_store.set_ref(&key, &value)?;
        
        println!("   Also inserted user {} version 1", user_id);
    }
    
    println!("\n2. Testing get_fuzzy_range_leq_kv with fuzzy_bytes = 0:");
    
    // Query for user 1000, version 8
    let mut query_key = table_type.to_be_bytes().to_vec();
    query_key.extend_from_slice(&1000u64.to_be_bytes());
    query_key.extend_from_slice(&8u32.to_be_bytes());
    
    println!("   Query key: {:?}", query_key);
    println!("   Expected: Only exact matches with key <= query_key");
    
    let mdbx_range = mdbx_store.get_fuzzy_range_leq_kv(&query_key, 0)?;
    let scylla_range = scylla_store.get_fuzzy_range_leq_kv(&query_key, 0)?;
    
    println!("\n   LibMDBX results ({} items):", mdbx_range.len());
    for (i, item) in mdbx_range.iter().enumerate() {
        println!("     [{}] key: {:?}, value: {}", 
                 i, item.key, String::from_utf8_lossy(&item.value));
    }
    
    println!("\n   ScyllaDB results ({} items):", scylla_range.len());
    for (i, item) in scylla_range.iter().enumerate() {
        println!("     [{}] key: {:?}, value: {}", 
                 i, item.key, String::from_utf8_lossy(&item.value));
    }
    
    println!("\n3. Analysis:");
    if mdbx_range.len() != scylla_range.len() {
        println!("   ❌ Different result counts!");
        println!("   LibMDBX seems to be returning only keys that exactly match the query");
        println!("   ScyllaDB might be returning all keys <= query_key");
        
        // Check what fuzzy_bytes=0 means
        println!("\n   Understanding fuzzy_bytes = 0:");
        println!("   - Should return all keys where key <= query_key");
        println!("   - NOT just exact matches");
        
        // Let's verify by checking the keys
        println!("\n   Checking if all ScyllaDB results are <= query_key:");
        let all_valid = scylla_range.iter().all(|item| item.key <= query_key);
        println!("   All valid: {}", all_valid);
    }
    
    println!("\n4. Testing with fuzzy_bytes = 4:");
    
    let mdbx_range_4 = mdbx_store.get_fuzzy_range_leq_kv(&query_key, 4)?;
    let scylla_range_4 = scylla_store.get_fuzzy_range_leq_kv(&query_key, 4)?;
    
    println!("   LibMDBX: {} results", mdbx_range_4.len());
    println!("   ScyllaDB: {} results", scylla_range_4.len());
    
    if mdbx_range_4.len() == scylla_range_4.len() {
        println!("   ✅ Results match with fuzzy_bytes = 4");
    }
    
    Ok(())
}