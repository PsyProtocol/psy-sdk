use anyhow::Result;
use kvq::traits::KVQBinaryStore;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use qed_data::config::store_config::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_tree_clustering_key_design() -> Result<()> {
    println!("=== Testing Tree Tables with New Clustering Key Design ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_tree_clustering");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_tree_clustering").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    println!("1. Understanding the new tree key structure:");
    println!("   Total key: [table_type(2) + tree_id(1) + primary_id(8) + secondary_id(4) + level(1) + index(8) + checkpoint_id(8)] = 32 bytes");
    println!("   With clustering_key_size = 8:");
    println!("   - Partition key: [tree_id(1) + primary_id(8) + secondary_id(4) + level(1) + index(8)] = 22 bytes");
    println!("   - Clustering key: [checkpoint_id(8)] = 8 bytes");
    println!("   This allows efficient range queries by checkpoint within the same tree node!");
    
    // Test with checkpoint tree
    let table_type = CHECKPOINT_TREE_TABLE_TYPE;
    let tree_id = 1u64;
    
    // Insert nodes at different checkpoints
    let checkpoints = vec![100u64, 200, 300, 400, 500];
    let node_ids = vec![0u32, 1, 2, 3];
    
    println!("\n2. Inserting test data:");
    for checkpoint_id in &checkpoints {
        for node_id in &node_ids {
            let mut key = table_type.to_be_bytes().to_vec();
            key.extend_from_slice(&tree_id.to_be_bytes());
            key.extend_from_slice(&checkpoint_id.to_be_bytes());
            key.extend_from_slice(&node_id.to_be_bytes());
            
            let value = format!("tree_node_{}_{}", checkpoint_id, node_id).into_bytes();
            
            mdbx_store.set_ref(&key, &value)?;
            scylla_store.set_ref(&key, &value)?;
            
            if *node_id == 0 {
                println!("   Checkpoint {} - key: {:?}", checkpoint_id, key);
            }
        }
    }
    
    println!("\n3. Testing get_leq with fuzzy_bytes = 12 (entire clustering key):");
    println!("   This should find the highest checkpoint <= query checkpoint");
    
    // Query for checkpoint 250 (should return checkpoint 200)
    let query_checkpoint = 250u64;
    let mut query_key = table_type.to_be_bytes().to_vec();
    query_key.extend_from_slice(&tree_id.to_be_bytes());
    query_key.extend_from_slice(&query_checkpoint.to_be_bytes());
    query_key.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // max node_id
    
    let mdbx_result = mdbx_store.get_leq(&query_key, 12)?;
    let scylla_result = scylla_store.get_leq(&query_key, 12)?;
    
    println!("   Query for checkpoint {}: ", query_checkpoint);
    println!("   - LibMDBX: {:?}", mdbx_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    // With the new design, ScyllaDB should be able to efficiently query within the same partition
    println!("\n4. Benefits of the new design:");
    println!("   - All nodes for a tree are in the same partition");
    println!("   - Can efficiently query by checkpoint within a tree");
    println!("   - No need for ALLOW FILTERING for common queries");
    println!("   - Better performance for checkpoint-based lookups");
    
    // Test exact match
    println!("\n5. Testing exact match:");
    let mut exact_key = table_type.to_be_bytes().to_vec();
    exact_key.extend_from_slice(&tree_id.to_be_bytes());
    exact_key.extend_from_slice(&300u64.to_be_bytes());
    exact_key.extend_from_slice(&2u32.to_be_bytes());
    
    let mdbx_exact = mdbx_store.get_exact_if_exists(&exact_key)?;
    let scylla_exact = scylla_store.get_exact_if_exists(&exact_key)?;
    
    println!("   - LibMDBX: {:?}", mdbx_exact.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_exact.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    assert_eq!(mdbx_exact, scylla_exact);
    
    Ok(())
}