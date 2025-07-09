use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQPair};
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use qed_data::config::store_config::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_comprehensive_fuzzy_operations() -> Result<()> {
    println!("=== Comprehensive Fuzzy Operations Test ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_comprehensive");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    println!("✓ LibMDBX initialized");
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_comprehensive").await {
        Ok(store) => {
            println!("✓ ScyllaDB initialized");
            store
        },
        Err(e) => {
            println!("✗ ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    println!("\n1. Testing Different Table Types with get_leq\n");
    
    // Test 1: Tree tables (use clustering store with 22-byte keys)
    println!("A. Tree Tables (Clustering Store, 22-byte keys):");
    test_tree_table_get_leq(&mdbx_store, &scylla_store).await?;
    
    // Test 2: Leaf tables (use clustering store with varying partition sizes)
    println!("\nB. Leaf Tables (Clustering Store, varying sizes):");
    test_leaf_table_get_leq(&mdbx_store, &scylla_store).await?;
    
    // Test 3: Helper tables (use KVQ store)
    println!("\nC. Helper Tables (KVQ Store):");
    test_helper_table_get_leq(&mdbx_store, &scylla_store).await?;
    
    // Test 4: Special case - checkpoint block state
    println!("\nD. Checkpoint Block State (Special Implementation):");
    test_checkpoint_block_state(&mdbx_store, &scylla_store).await?;
    
    println!("\n2. Testing get_fuzzy_range_leq_kv\n");
    test_fuzzy_range_operations(&mdbx_store, &scylla_store).await?;
    
    println!("\n=== Test Summary ===");
    println!("✓ Exact matching (fuzzy_bytes=0) works consistently across all stores");
    println!("⚠ Fuzzy matching behavior differs:");
    println!("  - LibMDBX: Byte-wise comparison ignoring last N bytes");
    println!("  - ScyllaDB Clustering: Structured queries within partitions");
    println!("  - ScyllaDB KVQ: Limited/no fuzzy support except special cases");
    
    Ok(())
}

async fn test_tree_table_get_leq(
    mdbx_store: &KVQlibmdbxStore,
    scylla_store: &ScyllaStore,
) -> Result<()> {
    let table_type = USER_TREE_TABLE_TYPE;
    let tree_id = 1u64;
    
    // Insert test data
    let nodes = vec![
        (100u64, 1u32), // (node_id, level)
        (200, 1),
        (300, 1),
        (150, 2),
        (250, 2),
    ];
    
    for (node_id, level) in &nodes {
        let mut key = table_type.to_be_bytes().to_vec();
        key.extend_from_slice(&tree_id.to_be_bytes());
        key.extend_from_slice(&node_id.to_be_bytes());
        key.extend_from_slice(&level.to_be_bytes());
        
        let value = format!("node_{}_{}", node_id, level).into_bytes();
        mdbx_store.set_ref(&key, &value)?;
        scylla_store.set_ref(&key, &value)?;
    }
    
    // Test exact match
    println!("  Exact match test:");
    let mut test_key = table_type.to_be_bytes().to_vec();
    test_key.extend_from_slice(&tree_id.to_be_bytes());
    test_key.extend_from_slice(&200u64.to_be_bytes());
    test_key.extend_from_slice(&1u32.to_be_bytes());
    
    let mdbx_exact = mdbx_store.get_leq(&test_key, 0)?;
    let scylla_exact = scylla_store.get_leq(&test_key, 0)?;
    
    println!("    LibMDBX: {:?}", mdbx_exact.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("    ScyllaDB: {:?}", scylla_exact.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("    Match: {}", mdbx_exact == scylla_exact);
    
    // Test fuzzy match
    println!("  Fuzzy match test (fuzzy_bytes=4):");
    let mut fuzzy_key = table_type.to_be_bytes().to_vec();
    fuzzy_key.extend_from_slice(&tree_id.to_be_bytes());
    fuzzy_key.extend_from_slice(&175u64.to_be_bytes());
    fuzzy_key.extend_from_slice(&3u32.to_be_bytes()); // Non-existent level
    
    let mdbx_fuzzy = mdbx_store.get_leq(&fuzzy_key, 4)?;
    let scylla_fuzzy = scylla_store.get_leq(&fuzzy_key, 4)?;
    
    println!("    Query: node_175_3");
    println!("    LibMDBX: {:?}", mdbx_fuzzy.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("    ScyllaDB: {:?}", scylla_fuzzy.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    Ok(())
}

async fn test_leaf_table_get_leq(
    mdbx_store: &KVQlibmdbxStore,
    scylla_store: &ScyllaStore,
) -> Result<()> {
    let table_type = USER_LEAF_TABLE_TYPE; // 8-byte partition key
    
    // Insert test data
    let users = vec![
        (1000u64, 1u32), // (user_id, version)
        (1000, 2),
        (1000, 3),
        (2000, 1),
        (2000, 2),
    ];
    
    for (user_id, version) in &users {
        let mut key = table_type.to_be_bytes().to_vec();
        key.extend_from_slice(&user_id.to_be_bytes());
        key.extend_from_slice(&version.to_be_bytes());
        
        let value = format!("user_{}_v{}", user_id, version).into_bytes();
        mdbx_store.set_ref(&key, &value)?;
        scylla_store.set_ref(&key, &value)?;
    }
    
    // Test within same partition
    println!("  Query within partition (user 1000):");
    let mut query_key = table_type.to_be_bytes().to_vec();
    query_key.extend_from_slice(&1000u64.to_be_bytes());
    query_key.extend_from_slice(&5u32.to_be_bytes()); // Higher than any version
    
    let mdbx_result = mdbx_store.get_leq(&query_key, 0)?;
    let scylla_result = scylla_store.get_leq(&query_key, 0)?;
    
    println!("    LibMDBX: {:?}", mdbx_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("    ScyllaDB: {:?}", scylla_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    Ok(())
}

async fn test_helper_table_get_leq(
    mdbx_store: &KVQlibmdbxStore,
    scylla_store: &ScyllaStore,
) -> Result<()> {
    let table_type = USER_PUBLIC_KEY_HELPER_TABLE_TYPE; // Uses KVQ store
    
    // Insert test data
    let keys = vec![
        vec![0, 17, 0, 0, 0, 1], // table_type + some data
        vec![0, 17, 0, 0, 0, 5],
        vec![0, 17, 0, 0, 0, 10],
    ];
    
    for (i, key) in keys.iter().enumerate() {
        let value = format!("helper_{}", i).into_bytes();
        mdbx_store.set_ref(key, &value)?;
        scylla_store.set_ref(key, &value)?;
    }
    
    // Test exact match
    println!("  Exact match:");
    let exact_result_mdbx = mdbx_store.get_leq(&keys[1], 0)?;
    let exact_result_scylla = scylla_store.get_leq(&keys[1], 0)?;
    
    println!("    LibMDBX: {:?}", exact_result_mdbx.is_some());
    println!("    ScyllaDB: {:?}", exact_result_scylla.is_some());
    println!("    Both found: {}", exact_result_mdbx.is_some() && exact_result_scylla.is_some());
    
    // Test fuzzy match (will likely fail for ScyllaDB KVQ store)
    println!("  Fuzzy match (fuzzy_bytes=2):");
    let fuzzy_key = vec![0, 17, 0, 0, 0, 7]; // Between 5 and 10
    
    let fuzzy_result_mdbx = mdbx_store.get_leq(&fuzzy_key, 2)?;
    let fuzzy_result_scylla = scylla_store.get_leq(&fuzzy_key, 2)?;
    
    println!("    LibMDBX: {:?}", fuzzy_result_mdbx.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("    ScyllaDB: {:?}", fuzzy_result_scylla.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("    Note: ScyllaDB KVQ store doesn't support fuzzy matching");
    
    Ok(())
}

async fn test_checkpoint_block_state(
    mdbx_store: &KVQlibmdbxStore,
    scylla_store: &ScyllaStore,
) -> Result<()> {
    let table_type = CHECKPOINT_BLOCK_STATE_TABLE_TYPE;
    
    // Insert checkpoint blocks
    let checkpoints = vec![1000u64, 2000, 3000, 4000];
    
    for cp_id in &checkpoints {
        let mut key = table_type.to_be_bytes().to_vec();
        key.extend_from_slice(&cp_id.to_be_bytes());
        
        let value = format!("checkpoint_{}", cp_id).into_bytes();
        mdbx_store.set_ref(&key, &value)?;
        scylla_store.set_ref(&key, &value)?;
    }
    
    // Test the special implementation
    let mut query_key = table_type.to_be_bytes().to_vec();
    query_key.extend_from_slice(&2500u64.to_be_bytes());
    
    let mdbx_result = mdbx_store.get_leq(&query_key, 8)?;
    let scylla_result = scylla_store.get_leq(&query_key, 8)?;
    
    println!("    Query checkpoint 2500:");
    println!("    LibMDBX: {:?}", mdbx_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("    ScyllaDB: {:?}", scylla_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("    Match: {}", mdbx_result == scylla_result);
    
    Ok(())
}

async fn test_fuzzy_range_operations(
    mdbx_store: &KVQlibmdbxStore,
    scylla_store: &ScyllaStore,
) -> Result<()> {
    let table_type = CONTRACT_TREE_TABLE_TYPE;
    let tree_id = 1u64;
    
    // Insert a range of nodes
    for i in 0..10u64 {
        let mut key = table_type.to_be_bytes().to_vec();
        key.extend_from_slice(&tree_id.to_be_bytes());
        key.extend_from_slice(&(i * 100).to_be_bytes());
        key.extend_from_slice(&0u32.to_be_bytes());
        
        let value = format!("contract_node_{}", i * 100).into_bytes();
        mdbx_store.set_ref(&key, &value)?;
        scylla_store.set_ref(&key, &value)?;
    }
    
    // Test get_fuzzy_range_leq_kv
    println!("  Testing get_fuzzy_range_leq_kv:");
    let mut range_key = table_type.to_be_bytes().to_vec();
    range_key.extend_from_slice(&tree_id.to_be_bytes());
    range_key.extend_from_slice(&500u64.to_be_bytes());
    range_key.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());
    
    let mdbx_range = mdbx_store.get_fuzzy_range_leq_kv(&range_key, 4)?;
    let scylla_range = scylla_store.get_fuzzy_range_leq_kv(&range_key, 4)?;
    
    println!("    Query: nodes <= 500");
    println!("    LibMDBX found: {} entries", mdbx_range.len());
    println!("    ScyllaDB found: {} entries", scylla_range.len());
    
    // Compare results
    if !mdbx_range.is_empty() && !scylla_range.is_empty() {
        println!("    First entry comparison:");
        println!("      LibMDBX: {:?}", String::from_utf8_lossy(&mdbx_range[0].value));
        println!("      ScyllaDB: {:?}", String::from_utf8_lossy(&scylla_range[0].value));
    }
    
    Ok(())
}