use kvq::traits::KVQPair;
use anyhow::Result;
use kvq::traits::KVQBinaryStore;
use kvq::traits::KVQBinaryStoreAsync;
use psy_store::store::lmdbx::KVQlibmdbxStore;
use psy_store::store::scylla::ScyllaStore;
use psy_data::config::store_config::*;
use std::sync::Arc;

// Helper function to construct keys based on table type
fn construct_key(table_type: u16, key_suffix: &[u8]) -> Vec<u8> {
    let mut key = table_type.to_be_bytes().to_vec();
    
    // Determine partition key size based on table type
    let partition_key_size = match table_type {
        // Tree tables use 22 bytes total
        CHECKPOINT_TREE_TABLE_TYPE | USER_TREE_TABLE_TYPE | CONTRACT_TREE_TABLE_TYPE |
        CONTRACT_FUNCTION_TREE_TABLE_TYPE | DEPOSIT_TREE_TABLE_TYPE | WITHDRAWAL_TREE_TABLE_TYPE |
        USER_REGISTRATION_TREE_TABLE_TYPE | USER_CONTRACT_TREE_TABLE_TYPE | 
        USER_CONTRACT_STATE_TREE_TABLE_TYPE => 22,
        
        // User and contract leaves use 8 bytes partition key
        USER_LEAF_TABLE_TYPE | CONTRACT_LEAF_TABLE_TYPE | CONTRACT_CODE_TABLE_TYPE => 8,
        
        // Checkpoint-related tables use 2 bytes partition key
        CHECKPOINT_LEAF_TABLE_TYPE | CHECKPOINT_BLOCK_STATE_TABLE_TYPE | 
        CHECKPOINT_SYNC_INFO_TABLE_TYPE => 2,
        
        // Default case - should not happen in production
        _ => 10,
    };
    
    // For clustering stores, we need to ensure the key is at least partition_key_size
    // The first 2 bytes are the table type, extend as needed
    if partition_key_size > 2 {
        key.extend_from_slice(&vec![0x00; partition_key_size - 2]);
    }
    
    // Add the key suffix
    key.extend_from_slice(key_suffix);
    
    key
}

// Use multi-threaded runtime to avoid block_in_place issues
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_mdbx_scylla_consistency_fixed() -> Result<()> {
    println!("=== Testing LibMDBX and ScyllaDB Consistency (Fixed Key Lengths) ===\n");
    
    // Setup
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_consistency_fixed");
    
    // Initialize libmdbx store
    println!("1. Initializing stores...");
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    println!("   ✓ LibMDBX initialized");
    
    // Initialize ScyllaDB store
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_consistency_fixed").await {
        Ok(store) => {
            println!("   ✓ ScyllaDB initialized");
            store
        },
        Err(e) => {
            println!("   ✗ ScyllaDB not available: {:?}", e);
            println!("   ⚠️  Skipping consistency tests. Please ensure ScyllaDB is running.");
            return Ok(());
        }
    };
    
    println!("\n2. Testing Write Consistency");
    println!("   Writing identical data to both stores...\n");
    
    // Test data for different table types with proper key construction
    let test_data = vec![
        // (table_name, table_type, key_suffix, value)
        // Tree tables - need longer keys (total 32 bytes = 2 + 22 + 8)
        ("Checkpoint Tree", CHECKPOINT_TREE_TABLE_TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01], vec![0xAA, 0xBB, 0xCC, 0xDD]),
        ("User Tree", USER_TREE_TABLE_TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02], vec![0x11, 0x22, 0x33, 0x44]),
        ("Contract Tree", CONTRACT_TREE_TABLE_TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03], vec![0x55, 0x66, 0x77, 0x88]),
        
        // User/Contract leaves - need 14 bytes total (2 + 8 + 4)
        ("User Leaf", USER_LEAF_TABLE_TYPE, vec![0x00, 0x00, 0x00, 0x04], vec![0x99, 0xAA, 0xBB, 0xCC]),
        ("Contract Leaf", CONTRACT_LEAF_TABLE_TYPE, vec![0x00, 0x00, 0x00, 0x06], vec![0x12, 0x34, 0x56, 0x78]),
        ("Contract Code", CONTRACT_CODE_TABLE_TYPE, vec![0x00, 0x00, 0x00, 0x07], vec![0x9A, 0xBC, 0xDE, 0xF0]),
        
        // Checkpoint-related tables - need 10 bytes total (2 + 8)
        ("Checkpoint Leaf", CHECKPOINT_LEAF_TABLE_TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05], vec![0xDD, 0xEE, 0xFF, 0x00]),
        ("Checkpoint Block State", CHECKPOINT_BLOCK_STATE_TABLE_TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08], vec![0x1A, 0x2B, 0x3C, 0x4D]),
        ("Checkpoint Sync Info", CHECKPOINT_SYNC_INFO_TABLE_TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09], vec![0x2A, 0x3B, 0x4C, 0x5D]),
    ];
    
    // Write to both stores and verify
    for (table_name, table_type, key_suffix, value) in &test_data {
        let key = construct_key(*table_type, key_suffix);
        
        println!("   Table: {} (type={})", table_name, table_type);
        println!("   Key:   {:?} (len={})", key, key.len());
        println!("   Value: {:?}", value);
        
        // Write to both stores
        mdbx_store.set_ref(&key, value)?;
        <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla_store, &key, value).await?;
        
        // Immediately read back and verify
        let mdbx_read = mdbx_store.get_exact(&key)?;
        let scylla_read = <ScyllaStore as KVQBinaryStoreAsync>::get_exact(&scylla_store, &key).await?;
        
        assert_eq!(mdbx_read, *value, "LibMDBX write/read mismatch");
        assert_eq!(scylla_read, *value, "ScyllaDB write/read mismatch");
        assert_eq!(mdbx_read, scylla_read, "Stores are not consistent!");
        
        println!("   ✓ Write verified - both stores consistent\n");
    }
    
    println!("3. Testing Read Consistency");
    println!("   Reading all data and comparing...\n");
    
    for (table_name, table_type, key_suffix, expected_value) in &test_data {
        let key = construct_key(*table_type, key_suffix);
        
        let mdbx_value = mdbx_store.get_exact(&key)?;
        let scylla_value = <ScyllaStore as KVQBinaryStoreAsync>::get_exact(&scylla_store, &key).await?;
        
        println!("   {} - LibMDBX: {:?}, ScyllaDB: {:?}", 
                 table_name, mdbx_value, scylla_value);
        
        assert_eq!(mdbx_value, *expected_value, "LibMDBX value mismatch");
        assert_eq!(scylla_value, *expected_value, "ScyllaDB value mismatch");
        assert_eq!(mdbx_value, scylla_value, "Read consistency failed!");
    }
    
    println!("\n   ✓ All reads are consistent!\n");
    
    println!("4. Testing Update Consistency");
    println!("   Updating values in both stores...\n");
    
    // Update some values
    let updates = vec![
        (USER_TREE_TABLE_TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02], vec![0xFF, 0xEE, 0xDD, 0xCC]),
        (CONTRACT_TREE_TABLE_TYPE, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03], vec![0xAA, 0xBB, 0xCC, 0xDD]),
    ];
    
    for (table_type, key_suffix, new_value) in &updates {
        let key = construct_key(*table_type, key_suffix);
        
        // Update both stores
        mdbx_store.set_ref(&key, new_value)?;
        <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla_store, &key, new_value).await?;
        
        // Verify updates
        let mdbx_updated = mdbx_store.get_exact(&key)?;
        let scylla_updated = <ScyllaStore as KVQBinaryStoreAsync>::get_exact(&scylla_store, &key).await?;
        
        assert_eq!(mdbx_updated, *new_value);
        assert_eq!(scylla_updated, *new_value);
        assert_eq!(mdbx_updated, scylla_updated);
        
        println!("   ✓ Update verified for table type {}", table_type);
    }
    
    println!("\n5. Testing Delete Consistency");
    println!("   Deleting from both stores...\n");
    
    // Delete a key
    let delete_key = construct_key(CHECKPOINT_LEAF_TABLE_TYPE, &vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05]);
    
    let mdbx_deleted = mdbx_store.delete(&delete_key)?;
    let scylla_deleted = <ScyllaStore as KVQBinaryStoreAsync>::delete(&scylla_store, &delete_key).await?;
    
    println!("   LibMDBX delete result: {}", mdbx_deleted);
    println!("   ScyllaDB delete result: {}", scylla_deleted);
    assert_eq!(mdbx_deleted, scylla_deleted, "Delete results not consistent");
    
    // Verify deletion
    let mdbx_after = mdbx_store.get_exact_if_exists(&delete_key)?;
    let scylla_after = <ScyllaStore as KVQBinaryStoreAsync>::get_exact_if_exists(&scylla_store, &delete_key).await?;
    
    assert!(mdbx_after.is_none(), "LibMDBX: key should be deleted");
    assert!(scylla_after.is_none(), "ScyllaDB: key should be deleted");
    println!("   ✓ Deletion verified - both stores consistent\n");
    
    println!("6. Testing Batch Operations Consistency");
    println!("   Performing batch writes...\n");
    
    let batch_data = vec![
        (construct_key(USER_TREE_TABLE_TYPE, &vec![0x10, 0x00]), vec![0x10, 0x20]),
        (construct_key(USER_TREE_TABLE_TYPE, &vec![0x10, 0x01]), vec![0x30, 0x40]),
        (construct_key(USER_TREE_TABLE_TYPE, &vec![0x10, 0x02]), vec![0x50, 0x60]),
        (construct_key(CONTRACT_TREE_TABLE_TYPE, &vec![0x20, 0x00]), vec![0x70, 0x80]),
        (construct_key(CONTRACT_TREE_TABLE_TYPE, &vec![0x20, 0x01]), vec![0x90, 0xA0]),
    ];
    
    let kvq_pairs: Vec<_> = batch_data.iter()
        .map(|(k, v)| kvq::traits::KVQPair { key: k.clone(), value: v.clone() })
        .collect();
    
    // Write batch to both stores
    mdbx_store.set_many_vec(kvq_pairs.clone())?;
    <ScyllaStore as KVQBinaryStoreAsync>::set_many_vec(&scylla_store, kvq_pairs).await?;
    
    // Verify all batch writes
    for (key, expected_value) in &batch_data {
        let mdbx_batch = mdbx_store.get_exact(key)?;
        let scylla_batch = <ScyllaStore as KVQBinaryStoreAsync>::get_exact(&scylla_store, key).await?;
        
        assert_eq!(mdbx_batch, *expected_value);
        assert_eq!(scylla_batch, *expected_value);
        assert_eq!(mdbx_batch, scylla_batch);
    }
    
    println!("   ✓ Batch operations consistent ({} items)", batch_data.len());
    
    println!("\n7. Testing Edge Cases");
    
    // Test empty value
    println!("   Testing empty values...");
    let empty_key = construct_key(USER_LEAF_TABLE_TYPE, &vec![0xFF, 0xFF]);
    let empty_value = vec![];
    
    mdbx_store.set_ref(&empty_key, &empty_value)?;
    <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla_store, &empty_key, &empty_value).await?;
    
    let mdbx_empty = mdbx_store.get_exact(&empty_key)?;
    let scylla_empty = <ScyllaStore as KVQBinaryStoreAsync>::get_exact(&scylla_store, &empty_key).await?;
    
    assert_eq!(mdbx_empty, empty_value);
    assert_eq!(scylla_empty, empty_value);
    assert_eq!(mdbx_empty, scylla_empty);
    println!("   ✓ Empty values handled consistently");
    
    // Test large value
    println!("   Testing large values...");
    let large_key = construct_key(CONTRACT_CODE_TABLE_TYPE, &vec![0xAA, 0xBB]);
    let large_value = vec![0xAB; 10000]; // 10KB
    
    mdbx_store.set_ref(&large_key, &large_value)?;
    <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla_store, &large_key, &large_value).await?;
    
    let mdbx_large = mdbx_store.get_exact(&large_key)?;
    let scylla_large = <ScyllaStore as KVQBinaryStoreAsync>::get_exact(&scylla_store, &large_key).await?;
    
    assert_eq!(mdbx_large.len(), 10000);
    assert_eq!(scylla_large.len(), 10000);
    assert_eq!(mdbx_large, scylla_large);
    println!("   ✓ Large values handled consistently");
    
    // Test non-existent key
    println!("   Testing non-existent keys...");
    let non_existent = construct_key(USER_TREE_TABLE_TYPE, &vec![0xFF; 10]);
    
    let mdbx_none = mdbx_store.get_exact_if_exists(&non_existent)?;
    let scylla_none = <ScyllaStore as KVQBinaryStoreAsync>::get_exact_if_exists(&scylla_store, &non_existent).await?;
    
    assert!(mdbx_none.is_none());
    assert!(scylla_none.is_none());
    println!("   ✓ Non-existent keys handled consistently");
    
    println!("\n✅ ALL CONSISTENCY TESTS PASSED!");
    println!("   LibMDBX and ScyllaDB are behaving identically for all operations.");
    
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_concurrent_consistency() -> Result<()> {
    use tokio::task;
    use std::sync::Arc;
    
    println!("=== Testing Concurrent Operations Consistency ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_concurrent");
    
    let mdbx_store = Arc::new(KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?);
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_concurrent").await {
        Ok(store) => Arc::new(store),
        Err(e) => {
            println!("ScyllaDB not available: {:?}. Skipping concurrent tests.", e);
            return Ok(());
        }
    };
    
    // Spawn multiple tasks to write concurrently
    let mut handles = vec![];
    
    for i in 0..10 {
        let mdbx = mdbx_store.clone();
        let scylla = scylla_store.clone();
        
        let handle = task::spawn(async move {
            let key = construct_key(USER_TREE_TABLE_TYPE, &vec![0x50, i]);
            let value = vec![i * 10, i * 10 + 1, i * 10 + 2];
            
            // Write to both stores
            mdbx.set_ref(&key, &value)?;
            <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla, &key, &value).await?;
            
            Ok::<(Vec<u8>, Vec<u8>), anyhow::Error>((key, value))
        });
        
        handles.push(handle);
    }
    
    // Wait for all writes to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // Verify all concurrent writes are consistent
    println!("Verifying concurrent writes...");
    for (i, result) in results.iter().enumerate() {
        let (key, expected_value) = result.as_ref().unwrap().as_ref().unwrap();
        
        let mdbx_value = mdbx_store.get_exact(key)?;
        let scylla_value = <ScyllaStore as KVQBinaryStoreAsync>::get_exact(&scylla_store, key).await?;
        
        assert_eq!(mdbx_value, *expected_value);
        assert_eq!(scylla_value, *expected_value);
        assert_eq!(mdbx_value, scylla_value);
        
        println!("   ✓ Task {} - values consistent", i);
    }
    
    println!("\n✅ Concurrent operations are consistent!");
    
    Ok(())
}