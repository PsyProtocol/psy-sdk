use anyhow::Result;
use kvq::traits::KVQBinaryStore;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use qed_store::config::store_config::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_checkpoint_tree_fixed() -> Result<()> {
    println!("=== Testing Checkpoint Tree with Fixed Implementation ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_checkpoint_fixed");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_checkpoint_fixed").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    // For checkpoint trees, the design might need reconsideration
    // Currently: partition_key_size = 22, which means NO clustering key
    // This defeats the purpose of using a clustering store
    
    // Better design would be:
    // - partition_key: [table_type (2) + tree_id (8)] = 10 bytes
    // - clustering_key: [height (8) + node_id (4)] = 12 bytes
    
    println!("Current design issue:");
    println!("- Checkpoint trees use partition_key_size = 22");
    println!("- This means the entire key is the partition key");
    println!("- No clustering key, so can't use efficient range queries");
    println!("- All queries require ALLOW FILTERING");
    
    println!("\nRecommended design:");
    println!("- partition_key: [table_type + tree_id] = 10 bytes");
    println!("- clustering_key: [height + node_id] = 12 bytes");
    println!("- This would allow efficient queries by height within a tree");
    
    // Test with current design
    let table_type = CHECKPOINT_TREE_TABLE_TYPE;
    
    // Create test data
    let heights = vec![100u64, 200, 300, 400, 500];
    
    for height in &heights {
        let mut key = table_type.to_be_bytes().to_vec();
        key.extend_from_slice(&1u64.to_be_bytes()); // tree_id = 1
        key.extend_from_slice(&height.to_be_bytes());
        key.extend_from_slice(&0u32.to_be_bytes());
        
        let value = format!("checkpoint_{}", height).into_bytes();
        
        mdbx_store.set_ref(&key, &value)?;
        scylla_store.set_ref(&key, &value)?;
    }
    
    // Test exact match
    println!("\n1. Testing exact match (should work):");
    let mut exact_key = table_type.to_be_bytes().to_vec();
    exact_key.extend_from_slice(&1u64.to_be_bytes());
    exact_key.extend_from_slice(&300u64.to_be_bytes());
    exact_key.extend_from_slice(&0u32.to_be_bytes());
    
    let mdbx_exact = mdbx_store.get_exact_if_exists(&exact_key)?;
    let scylla_exact = scylla_store.get_exact_if_exists(&exact_key)?;
    
    println!("   LibMDBX: {:?}", mdbx_exact.is_some());
    println!("   ScyllaDB: {:?}", scylla_exact.is_some());
    assert_eq!(mdbx_exact, scylla_exact);
    
    // Test with fuzzy_bytes that doesn't work well with current design
    println!("\n2. Testing fuzzy match (problematic with current design):");
    println!("   Note: This requires scanning all partitions with ALLOW FILTERING");
    
    Ok(())
}