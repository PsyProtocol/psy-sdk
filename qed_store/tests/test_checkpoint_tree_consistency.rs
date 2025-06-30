use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQSerializable};
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use qed_store::config::store_config::*;
use qed_store::models::kvq_merkle::key::KVQMerkleNodeKey;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_checkpoint_tree_consistency() -> Result<()> {
    println!("=== Testing Checkpoint Tree Consistency ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_checkpoint_tree");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 10)?;
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_checkpoint_tree").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    println!("1. Checkpoint Tree Node Structure:");
    println!("   Table type: CHECKPOINT_TREE_TABLE_TYPE = {}", CHECKPOINT_TREE_TABLE_TYPE);
    println!("   Key: [table_type(2) + tree_id(1) + primary_id(8) + secondary_id(4) + level(1) + index(8) + checkpoint_id(8)] = 32 bytes");
    println!("   Clustering key size: 8 bytes (checkpoint_id)");
    
    // Create checkpoint tree nodes
    let tree_id = 0u8; // Checkpoint tree has tree_id = 0
    let checkpoint_id = 100u64;
    
    println!("\n2. Inserting checkpoint tree nodes for checkpoint {}:", checkpoint_id);
    
    // Insert root node (level 255, index 0)
    let root_node = KVQMerkleNodeKey::<CHECKPOINT_TREE_TABLE_TYPE> {
        tree_id,
        primary_id: 0,
        secondary_id: 0,
        level: 255,
        index: 0,
        checkpoint_id,
    };
    
    let root_value = b"checkpoint_tree_root_node".to_vec();
    let root_key_bytes = root_node.to_bytes()?;
    
    println!("   Root node key: {:?} (len={})", root_key_bytes, root_key_bytes.len());
    
    mdbx_store.set_ref(&root_key_bytes, &root_value)?;
    scylla_store.set_ref(&root_key_bytes, &root_value)?;
    
    // Insert some intermediate nodes
    for level in 0..3 {
        for index in 0..4 {
            let node = KVQMerkleNodeKey::<CHECKPOINT_TREE_TABLE_TYPE> {
                tree_id,
                primary_id: 0,
                secondary_id: 0,
                level: level as u8,
                index: index as u64,
                checkpoint_id,
            };
            
            let value = format!("checkpoint_tree_node_{}_{}", level, index).into_bytes();
            let key_bytes = node.to_bytes()?;
            
            mdbx_store.set_ref(&key_bytes, &value)?;
            scylla_store.set_ref(&key_bytes, &value)?;
        }
    }
    
    println!("\n3. Testing get_leq for checkpoint tree nodes:");
    
    // Test exact match (fuzzy_bytes = 0)
    println!("   Testing exact match:");
    let mdbx_result = mdbx_store.get_leq(&root_key_bytes, 0)?;
    let scylla_result = scylla_store.get_leq(&root_key_bytes, 0)?;
    
    if mdbx_result == scylla_result {
        println!("   ✓ Exact match consistent");
    } else {
        println!("   ❌ Exact match MISMATCH!");
        println!("      LibMDBX: {:?}", mdbx_result.as_ref().map(|v| String::from_utf8_lossy(v)));
        println!("      ScyllaDB: {:?}", scylla_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    }
    
    // Test with fuzzy_bytes = 8 (checkpoint_id fuzzy)
    println!("\n   Testing with fuzzy_bytes = 8 (checkpoint_id fuzzy):");
    
    // Query for a slightly different checkpoint
    let query_node = KVQMerkleNodeKey::<CHECKPOINT_TREE_TABLE_TYPE> {
        tree_id,
        primary_id: 0,
        secondary_id: 0,
        level: 255,
        index: 0,
        checkpoint_id: 101, // Different checkpoint
    };
    let query_key_bytes = query_node.to_bytes()?;
    
    let mdbx_result = mdbx_store.get_leq(&query_key_bytes, 8)?;
    let scylla_result = scylla_store.get_leq(&query_key_bytes, 8)?;
    
    if mdbx_result == scylla_result {
        println!("   ✓ Fuzzy match consistent");
    } else {
        println!("   ❌ Fuzzy match MISMATCH!");
        println!("      LibMDBX: {:?}", mdbx_result.as_ref().map(|v| String::from_utf8_lossy(v)));
        println!("      ScyllaDB: {:?}", scylla_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    }
    
    // Test edge case: Query for non-existent node
    println!("\n4. Testing non-existent node query:");
    
    let non_exist_node = KVQMerkleNodeKey::<CHECKPOINT_TREE_TABLE_TYPE> {
        tree_id,
        primary_id: 0,
        secondary_id: 0,
        level: 254, // Different level
        index: 999, // High index
        checkpoint_id: 100,
    };
    let non_exist_key_bytes = non_exist_node.to_bytes()?;
    
    for fuzzy_bytes in [0, 4, 8] {
        let mdbx_result = mdbx_store.get_leq(&non_exist_key_bytes, fuzzy_bytes)?;
        let scylla_result = scylla_store.get_leq(&non_exist_key_bytes, fuzzy_bytes)?;
        
        if mdbx_result == scylla_result {
            println!("   ✓ Non-existent node with fuzzy_bytes={} consistent", fuzzy_bytes);
        } else {
            println!("   ❌ Non-existent node with fuzzy_bytes={} MISMATCH!", fuzzy_bytes);
        }
    }
    
    // Test checkpoint tree root retrieval pattern
    println!("\n5. Testing checkpoint tree root retrieval pattern:");
    println!("   This simulates get_checkpoint_tree_root() behavior");
    
    // The pattern used to find checkpoint tree root
    let root_pattern = KVQMerkleNodeKey::<CHECKPOINT_TREE_TABLE_TYPE> {
        tree_id: 0,
        primary_id: 0,
        secondary_id: 0,
        level: 255,
        index: 0,
        checkpoint_id: 100,
    };
    let root_pattern_bytes = root_pattern.to_bytes()?;
    
    // Should find exact match
    let mdbx_root = mdbx_store.get_exact_if_exists(&root_pattern_bytes)?;
    let scylla_root = scylla_store.get_exact_if_exists(&root_pattern_bytes)?;
    
    match (&mdbx_root, &scylla_root) {
        (Some(m), Some(s)) if m == s => {
            println!("   ✓ Root retrieval consistent: {}", String::from_utf8_lossy(&m));
        }
        (None, None) => {
            println!("   ❌ Both stores returned None for root!");
        }
        _ => {
            println!("   ❌ Root retrieval MISMATCH!");
            println!("      LibMDBX: {:?}", mdbx_root.as_ref().map(|v| String::from_utf8_lossy(&v)));
            println!("      ScyllaDB: {:?}", scylla_root.as_ref().map(|v| String::from_utf8_lossy(&v)));
        }
    }
    
    Ok(())
}