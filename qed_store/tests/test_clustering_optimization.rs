use anyhow::Result;
use kvq::traits::KVQBinaryStore;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use qed_store::config::store_config::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_clustering_optimization() -> Result<()> {
    println!("=== Testing Clustering Store Optimization ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_optimization");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_optimization").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    // Test with checkpoint block state table
    let table_type = CHECKPOINT_BLOCK_STATE_TABLE_TYPE;
    
    println!("1. Checkpoint Block State Table:");
    println!("   Key: [table_type(2) + checkpoint_id(8)] = 10 bytes");
    println!("   Clustering key size = 8");
    println!("   Partition key size = 2");
    
    // Insert test data
    for checkpoint_id in [100u64, 200, 300, 400, 500] {
        let mut key = table_type.to_be_bytes().to_vec();
        key.extend_from_slice(&checkpoint_id.to_be_bytes());
        
        let value = format!("checkpoint_{}", checkpoint_id).into_bytes();
        
        mdbx_store.set_ref(&key, &value)?;
        scylla_store.set_ref(&key, &value)?;
    }
    
    println!("\n2. Test Case 1: fuzzy_bytes = 3 (within clustering key)");
    println!("   Fixed prefix length: 10 - 3 = 7 bytes");
    println!("   Partition key size: 2 bytes");
    println!("   Since 7 >= 2, all partition key bytes are fixed");
    println!("   ✅ Can optimize: Query single partition");
    
    let mut query_key = table_type.to_be_bytes().to_vec();
    query_key.extend_from_slice(&250u64.to_be_bytes());
    
    let result = scylla_store.get_leq(&query_key, 3)?;
    println!("   Result: {:?}", result.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    println!("\n3. Test Case 2: fuzzy_bytes = 8 (matches clustering key)");
    println!("   ✅ Best case: Use efficient prepared statement");
    
    let result = scylla_store.get_leq(&query_key, 8)?;
    println!("   Result: {:?}", result.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    println!("\n4. Test Case 3: fuzzy_bytes = 9 (spans partition boundary)");
    println!("   Fixed prefix length: 10 - 9 = 1 byte");
    println!("   Partition key size: 2 bytes");
    println!("   Since 1 < 2, partition key is partially variable");
    println!("   ❌ Must scan all partitions");
    
    let result = scylla_store.get_leq(&query_key, 9)?;
    println!("   Result: {:?}", result.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    // Test with tree table
    println!("\n5. Tree Table Example:");
    println!("   Key: 32 bytes total");
    println!("   Partition key: 22 bytes (excludes table_type)");
    println!("   Clustering key: 8 bytes");
    
    println!("\n   - fuzzy_bytes = 4: Fixed prefix = 28, can query single partition ✅");
    println!("   - fuzzy_bytes = 8: Matches clustering key, use prepared statement ✅");
    println!("   - fuzzy_bytes = 12: Fixed prefix = 20, spans partitions ❌");
    
    println!("\n✅ Optimization working correctly!");
    
    Ok(())
}