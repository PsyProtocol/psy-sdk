use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync};
use psy_store::store::lmdbx::KVQlibmdbxStore;
use psy_store::store::scylla::ScyllaStore;
use psy_data::config::store_config::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_debug_checkpoint_tree() -> Result<()> {
    println!("=== Debug Checkpoint Tree Test ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_debug");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_debug").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    let table_type = CHECKPOINT_TREE_TABLE_TYPE;
    
    // Create a simple test case
    println!("1. Creating test data");
    
    // Key 1: height 100
    let mut key1 = table_type.to_be_bytes().to_vec();
    key1.extend_from_slice(&1u64.to_be_bytes()); // tree_id = 1
    key1.extend_from_slice(&100u64.to_be_bytes()); // height = 100
    key1.extend_from_slice(&0u32.to_be_bytes()); // node_id = 0
    
    let value1 = b"checkpoint_100".to_vec();
    
    println!("   Key1: {:?} (len={})", key1, key1.len());
    mdbx_store.set_ref(&key1, &value1)?;
    <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla_store, &key1, &value1).await?;
    
    // Query for height 150
    let mut query_key = table_type.to_be_bytes().to_vec();
    query_key.extend_from_slice(&1u64.to_be_bytes()); // tree_id = 1
    query_key.extend_from_slice(&150u64.to_be_bytes()); // height = 150
    query_key.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // max node_id
    
    println!("\n2. Testing queries");
    println!("   Query key: {:?} (len={})", query_key, query_key.len());
    
    // Test with different fuzzy_bytes values
    for fuzzy_bytes in &[0, 4, 8, 12] {
        println!("\n   Testing with fuzzy_bytes = {}", fuzzy_bytes);
        
        let mdbx_result = mdbx_store.get_leq(&query_key, *fuzzy_bytes)?;
        let scylla_result = <ScyllaStore as KVQBinaryStoreAsync>::get_leq(&scylla_store, &query_key, *fuzzy_bytes).await?;
        
        println!("     LibMDBX: {:?}", mdbx_result.as_ref().map(|v| String::from_utf8_lossy(v)));
        println!("     ScyllaDB: {:?}", scylla_result.as_ref().map(|v| String::from_utf8_lossy(v)));
        
        if *fuzzy_bytes > 0 {
            // Show key comparison details
            println!("\n     Key comparison (fuzzy_bytes={}):", fuzzy_bytes);
            println!("     key1 prefix:  {:?}", &key1[..key1.len().saturating_sub(*fuzzy_bytes)]);
            println!("     query prefix: {:?}", &query_key[..query_key.len().saturating_sub(*fuzzy_bytes)]);
            println!("     key1 suffix:  {:?}", &key1[key1.len().saturating_sub(*fuzzy_bytes)..]);
            println!("     query suffix: {:?}", &query_key[query_key.len().saturating_sub(*fuzzy_bytes)..]);
        }
    }
    
    Ok(())
}