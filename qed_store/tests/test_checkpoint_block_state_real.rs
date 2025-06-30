use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQSerializable};
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use qed_store::config::store_config::*;
use qed_data::qdata::u64_key::U64TableKey;

const CHECKPOINT_ID_FUZZY_SIZE: usize = 8;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_checkpoint_block_state_real_usage() -> Result<()> {
    println!("=== Testing Real Checkpoint Block State Usage ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_real_checkpoint");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_real_checkpoint").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    println!("1. Testing with U64TableKey serialization:");
    
    // Insert checkpoint block states using U64TableKey
    let checkpoint_ids = vec![100u64, 200, 300, 400, 500];
    
    for checkpoint_id in &checkpoint_ids {
        let key = U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(*checkpoint_id);
        let key_bytes = key.to_bytes()?;
        let value = format!("block_state_{}", checkpoint_id).into_bytes();
        
        println!("   Checkpoint {} - key bytes: {:?} (len={})", 
                 checkpoint_id, key_bytes, key_bytes.len());
        
        mdbx_store.set_ref(&key_bytes, &value)?;
        scylla_store.set_ref(&key_bytes, &value)?;
    }
    
    println!("\n2. Testing get_latest_block_state pattern:");
    
    // This is how get_latest_block_state works
    let max_key = U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(0xffffffffffffffu64);
    let max_key_bytes = max_key.to_bytes()?;
    
    println!("   Max key bytes: {:?}", max_key_bytes);
    println!("   Using fuzzy_bytes = {}", CHECKPOINT_ID_FUZZY_SIZE);
    
    let mdbx_latest = mdbx_store.get_leq(&max_key_bytes, CHECKPOINT_ID_FUZZY_SIZE)?;
    let scylla_latest = scylla_store.get_leq(&max_key_bytes, CHECKPOINT_ID_FUZZY_SIZE)?;
    
    println!("\n   Results:");
    println!("   - LibMDBX: {:?}", mdbx_latest.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_latest.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    if mdbx_latest != scylla_latest {
        println!("\n   ❌ ERROR: Results don't match!");
        println!("   This is the issue - get_leq returns different values");
    } else {
        println!("\n   ✅ Results match!");
    }
    
    // Test querying for a specific checkpoint
    println!("\n3. Testing query for checkpoint 250:");
    let query_key = U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(250);
    let query_bytes = query_key.to_bytes()?;
    
    let mdbx_result = mdbx_store.get_leq(&query_bytes, CHECKPOINT_ID_FUZZY_SIZE)?;
    let scylla_result = scylla_store.get_leq(&query_bytes, CHECKPOINT_ID_FUZZY_SIZE)?;
    
    println!("   - LibMDBX: {:?}", mdbx_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - Expected: block_state_200");
    
    // Debug the ScyllaDB clustering store behavior
    println!("\n4. Understanding ScyllaDB clustering store:");
    println!("   - Table uses partition_key_size = 2");
    println!("   - Partition key: table_type (2 bytes) = {:?}", &max_key_bytes[..2]);
    println!("   - Clustering key: checkpoint_id (8 bytes) = {:?}", &max_key_bytes[2..]);
    println!("   - With fuzzy_bytes = 8, we're looking for keys with:");
    println!("     * Same prefix (first 2 bytes = table type)");
    println!("     * Clustering key <= query clustering key");
    
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_debug_fuzzy_comparison() -> Result<()> {
    println!("=== Debug Fuzzy Comparison Logic ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_debug_fuzzy");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_debug_fuzzy").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    // Create a simple test case
    let key1 = vec![0x00, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64]; // checkpoint 100
    let key2 = vec![0x00, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC8]; // checkpoint 200
    let query = vec![0x00, 0x0C, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]; // max checkpoint
    
    mdbx_store.set_ref(&key1, &b"value1".to_vec())?;
    mdbx_store.set_ref(&key2, &b"value2".to_vec())?;
    scylla_store.set_ref(&key1, &b"value1".to_vec())?;
    scylla_store.set_ref(&key2, &b"value2".to_vec())?;
    
    println!("Keys inserted:");
    println!("  key1: {:?}", key1);
    println!("  key2: {:?}", key2);
    println!("  query: {:?}", query);
    
    println!("\nWith fuzzy_bytes = 8:");
    println!("  Prefix comparison (first 2 bytes):");
    println!("    key1 prefix: {:?}", &key1[..2]);
    println!("    key2 prefix: {:?}", &key2[..2]);
    println!("    query prefix: {:?}", &query[..2]);
    println!("    All prefixes match: ✓");
    
    println!("\n  Suffix comparison (last 8 bytes):");
    println!("    key1 suffix: {:?} = {}", &key1[2..], u64::from_be_bytes(key1[2..].try_into().unwrap()));
    println!("    key2 suffix: {:?} = {}", &key2[2..], u64::from_be_bytes(key2[2..].try_into().unwrap()));
    println!("    query suffix: {:?} = {}", &query[2..], u64::from_be_bytes(query[2..].try_into().unwrap()));
    
    let result = scylla_store.get_leq(&query, 8)?;
    println!("\n  ScyllaDB get_leq result: {:?}", result.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("  Expected: value2 (key2 has the largest suffix <= query suffix)");
    
    Ok(())
}