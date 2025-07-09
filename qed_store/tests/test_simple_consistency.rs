use anyhow::Result;
use kvq::traits::KVQBinaryStore;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_data::config::store_config::*;

// Test only libmdbx for now as ScyllaDB requires special runtime handling
#[test]
fn test_libmdbx_basic_operations() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_mdbx");
    
    println!("Testing libmdbx basic operations...");
    let store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    // Test different table types
    let test_cases = vec![
        (
            "Checkpoint Tree",
            CHECKPOINT_TREE_TABLE_TYPE,
            vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
            vec![0xAA, 0xBB, 0xCC, 0xDD]
        ),
        (
            "User Tree", 
            USER_TREE_TABLE_TYPE,
            vec![0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02],
            vec![0x11, 0x22, 0x33, 0x44]
        ),
        (
            "Contract Tree",
            CONTRACT_TREE_TABLE_TYPE,
            vec![0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03],
            vec![0x55, 0x66, 0x77, 0x88]
        ),
        (
            "User Leaf",
            USER_LEAF_TABLE_TYPE,
            vec![0x00, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04],
            vec![0x99, 0xAA, 0xBB, 0xCC]
        ),
        (
            "Checkpoint Leaf",
            CHECKPOINT_LEAF_TABLE_TYPE,
            vec![0x00, 0x0B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05],
            vec![0xDD, 0xEE, 0xFF, 0x00]
        ),
        (
            "Contract Leaf",
            CONTRACT_LEAF_TABLE_TYPE,
            vec![0x00, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06],
            vec![0x12, 0x34, 0x56, 0x78]
        ),
        (
            "Contract Code",
            CONTRACT_CODE_TABLE_TYPE,
            vec![0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07],
            vec![0x9A, 0xBC, 0xDE, 0xF0]
        ),
    ];
    
    // Write test
    println!("\n1. Testing WRITE operations:");
    for (name, table_type, key, value) in &test_cases {
        println!("   Writing to {} (table_type={})", name, table_type);
        store.set_ref(key, value)?;
        println!("   ✓ Written successfully");
    }
    
    // Read test
    println!("\n2. Testing READ operations:");
    for (name, _table_type, key, expected_value) in &test_cases {
        let value = store.get_exact(key)?;
        assert_eq!(value, *expected_value, "Value mismatch for {}", name);
        println!("   ✓ {} read correctly", name);
    }
    
    // Update test
    println!("\n3. Testing UPDATE operations:");
    let update_key = vec![0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02];
    let new_value = vec![0xFF, 0xEE, 0xDD, 0xCC];
    
    store.set_ref(&update_key, &new_value)?;
    let updated = store.get_exact(&update_key)?;
    assert_eq!(updated, new_value);
    println!("   ✓ Update successful");
    
    // Delete test
    println!("\n4. Testing DELETE operations:");
    let delete_key = vec![0x00, 0x0B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05];
    
    let deleted = store.delete(&delete_key)?;
    assert!(deleted, "Delete should return true");
    
    let after_delete = store.get_exact_if_exists(&delete_key)?;
    assert!(after_delete.is_none(), "Key should be deleted");
    println!("   ✓ Delete successful");
    
    // Batch operations test
    println!("\n5. Testing BATCH operations:");
    let batch_data = vec![
        (vec![0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00], vec![0x10, 0x20]),
        (vec![0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01], vec![0x30, 0x40]),
        (vec![0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02], vec![0x50, 0x60]),
    ];
    
    let kvq_pairs: Vec<_> = batch_data.iter()
        .map(|(k, v)| kvq::traits::KVQPair { key: k.clone(), value: v.clone() })
        .collect();
    
    store.set_many_vec(kvq_pairs)?;
    
    // Verify batch writes
    for (key, expected_value) in &batch_data {
        let value = store.get_exact(key)?;
        assert_eq!(value, *expected_value);
    }
    println!("   ✓ Batch operations successful");
    
    // Test fuzzy operations
    println!("\n6. Testing FUZZY operations:");
    let base_key = vec![0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00];
    
    // Write some test data with similar keys
    for i in 0..5 {
        let mut key = base_key.clone();
        key[9] = i;
        let value = vec![i * 10, i * 10 + 1];
        store.set_ref(&key, &value)?;
    }
    
    // Test get_leq (less than or equal)
    let search_key = vec![0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x03];
    let leq_result = store.get_leq(&search_key, 0)?;
    assert!(leq_result.is_some());
    println!("   ✓ Fuzzy operations successful");
    
    println!("\n✅ All libmdbx tests passed!");
    
    Ok(())
}

#[test] 
fn test_table_type_isolation() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_isolation");
    
    println!("Testing table type isolation in libmdbx...");
    let store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    // Same ID but different table types
    let same_id = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF];
    
    // Create keys with same ID but different table types
    let user_tree_key = [&USER_TREE_TABLE_TYPE.to_be_bytes()[..], &same_id[..]].concat();
    let contract_tree_key = [&CONTRACT_TREE_TABLE_TYPE.to_be_bytes()[..], &same_id[..]].concat();
    let checkpoint_tree_key = [&CHECKPOINT_TREE_TABLE_TYPE.to_be_bytes()[..], &same_id[..]].concat();
    
    let user_value = vec![0x11, 0x11];
    let contract_value = vec![0x22, 0x22];
    let checkpoint_value = vec![0x33, 0x33];
    
    // Write different values
    store.set_ref(&user_tree_key, &user_value)?;
    store.set_ref(&contract_tree_key, &contract_value)?;
    store.set_ref(&checkpoint_tree_key, &checkpoint_value)?;
    
    // Verify isolation
    let read_user = store.get_exact(&user_tree_key)?;
    let read_contract = store.get_exact(&contract_tree_key)?;
    let read_checkpoint = store.get_exact(&checkpoint_tree_key)?;
    
    assert_eq!(read_user, user_value);
    assert_eq!(read_contract, contract_value);
    assert_eq!(read_checkpoint, checkpoint_value);
    
    println!("✓ Table type isolation works correctly!");
    
    Ok(())
}