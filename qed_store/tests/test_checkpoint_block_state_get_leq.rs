use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync};
use qed_store::store::lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use psy_data::config::store_config::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_checkpoint_block_state_get_leq_issue() -> Result<()> {
    println!("=== Testing Checkpoint Block State get_leq Issue ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_checkpoint_issue");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_checkpoint_issue").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    // U64TableKey serializes to: [table_type (2 bytes) + u64 (8 bytes)] = 10 bytes total
    let table_type = CHECKPOINT_BLOCK_STATE_TABLE_TYPE;
    
    println!("1. Understanding the key structure:");
    println!("   - Table type: {} (0x{:04X})", table_type, table_type);
    println!("   - Key format: [table_type (2 bytes) + checkpoint_id (8 bytes)]");
    println!("   - Total key size: 10 bytes");
    println!("   - CHECKPOINT_ID_FUZZY_SIZE: 8 bytes");
    
    // Insert some checkpoint block states
    let checkpoint_ids = vec![100u64, 200, 300, 400, 500];
    
    for checkpoint_id in &checkpoint_ids {
        let mut key = table_type.to_be_bytes().to_vec();
        key.extend_from_slice(&checkpoint_id.to_be_bytes());
        
        let value = format!("block_state_{}", checkpoint_id).into_bytes();
        
        mdbx_store.set_ref(&key, &value)?;
        <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla_store, &key, &value).await?;
        
        println!("\n   Inserted checkpoint {} - key: {:?}", checkpoint_id, key);
    }
    
    println!("\n2. Testing get_leq with fuzzy_bytes = 8:");
    println!("   This should ignore the last 8 bytes when comparing");
    
    // Query for a non-existent checkpoint ID
    let query_id = 250u64;
    let mut query_key = table_type.to_be_bytes().to_vec();
    query_key.extend_from_slice(&query_id.to_be_bytes());
    
    println!("\n   Query key for checkpoint {}: {:?}", query_id, query_key);
    
    let mdbx_result = mdbx_store.get_leq(&query_key, 8)?;
    let scylla_result = <ScyllaStore as KVQBinaryStoreAsync>::get_leq(&scylla_store, &query_key, 8).await?;
    
    println!("\n   Results with fuzzy_bytes = 8:");
    println!("   - LibMDBX: {:?}", mdbx_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    // With fuzzy_bytes = 8, we're comparing only the first 2 bytes (table type)
    // So all keys with the same table type are considered equal in the prefix
    // Then we need to find the one with checkpoint_id <= 250
    // Expected: block_state_200
    
    println!("\n3. Key comparison analysis:");
    println!("   When fuzzy_bytes = 8:");
    println!("   - Only first 2 bytes (table type) are compared for prefix match");
    println!("   - Last 8 bytes (checkpoint_id) are compared for <= check");
    println!("   - Expected result: block_state_200 (largest checkpoint_id <= 250)");
    
    // Test with exact match
    println!("\n4. Testing exact match (fuzzy_bytes = 0):");
    let mut exact_key = table_type.to_be_bytes().to_vec();
    exact_key.extend_from_slice(&200u64.to_be_bytes());
    
    let mdbx_exact = mdbx_store.get_leq(&exact_key, 0)?;
    let scylla_exact = <ScyllaStore as KVQBinaryStoreAsync>::get_leq(&scylla_store, &exact_key, 0).await?;
    
    println!("   - LibMDBX: {:?}", mdbx_exact.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_exact.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    // Test get_latest_block_state pattern
    println!("\n5. Testing get_latest_block_state pattern:");
    let max_key = table_type.to_be_bytes().to_vec();
    let mut max_key_final = max_key.clone();
    max_key_final.extend_from_slice(&0xFFFFFFFFFFFFFFFFu64.to_be_bytes());
    
    let mdbx_latest = mdbx_store.get_leq(&max_key_final, 8)?;
    let scylla_latest = <ScyllaStore as KVQBinaryStoreAsync>::get_leq(&scylla_store, &max_key_final, 8).await?;
    
    println!("   Query with max checkpoint_id (0xFFFFFFFFFFFFFFFF):");
    println!("   - LibMDBX: {:?}", mdbx_latest.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_latest.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - Expected: block_state_500 (the latest checkpoint)");
    
    Ok(())
}