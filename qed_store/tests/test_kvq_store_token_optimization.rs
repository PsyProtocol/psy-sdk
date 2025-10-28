use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync};
use qed_store::store::lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use psy_data::config::store_config::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_kvq_store_token_analysis() -> Result<()> {
    println!("=== Analyzing Token Optimization for KVQ Store ===\n");
    
    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_kvq_token");
    
    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;
    
    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_kvq_token").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };
    
    println!("1. KVQ Store Structure:");
    println!("   - Table: (key blob PRIMARY KEY, value blob)");
    println!("   - No partition/clustering key split");
    println!("   - Entire key is the partition key");
    
    println!("\n2. Token Function Limitations:");
    println!("   - token() produces hash-based ordering");
    println!("   - Hash order ≠ lexicographic order");
    println!("   - Example: token(0x01) might be > token(0xFF)");
    
    println!("\n3. Fuzzy Matching Requirements:");
    println!("   - Need: key[:-fuzzy] == query[:-fuzzy] AND key[-fuzzy:] <= query[-fuzzy:]");
    println!("   - This requires byte-wise comparison");
    println!("   - Cannot be expressed using token ranges");
    
    // Test with user public key helper table
    let table_type = USER_PUBLIC_KEY_HELPER_TABLE_TYPE;
    
    // Insert some test data
    let user_id = 1000u64;
    for version in [1u32, 2, 3, 4, 5] {
        let mut key = table_type.to_be_bytes().to_vec();
        key.extend_from_slice(&user_id.to_be_bytes());
        key.extend_from_slice(&version.to_be_bytes());
        
        let value = format!("user_{}_v{}", user_id, version).into_bytes();
        
        mdbx_store.set_ref(&key, &value)?;
        <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla_store, &key, &value).await?;
    }
    
    println!("\n4. Example: User Public Key Helper");
    println!("   Key: [table_type(2) + user_id(8) + version(4)] = 14 bytes");
    
    // Query for version 3.5 (between 3 and 4)
    let mut query_key = table_type.to_be_bytes().to_vec();
    query_key.extend_from_slice(&user_id.to_be_bytes());
    query_key.extend_from_slice(&[0, 0, 0, 3]); // Between version 3 and 4
    query_key[13] = 128; // 3.5 in a way
    
    println!("\n5. With fuzzy_bytes = 4:");
    println!("   - Fixed prefix: first 10 bytes [table_type + user_id]");
    println!("   - Variable suffix: last 4 bytes [version]");
    
    let result = <ScyllaStore as KVQBinaryStoreAsync>::get_leq(&scylla_store, &query_key, 4).await?;
    println!("   Result: {:?}", result.as_ref().map(|v| String::from_utf8_lossy(v)));
    
    println!("\n6. Token Optimization Analysis:");
    println!("   ❌ Cannot use token ranges because:");
    println!("   - We need keys with specific prefix (first 10 bytes)");
    println!("   - Token of prefix doesn't help find all matching keys");
    println!("   - Would need to know all possible suffixes");
    
    println!("\n7. Potential Optimization Strategies:");
    println!("   a) Secondary Index on key prefix (but ScyllaDB doesn't support blob prefixes)");
    println!("   b) Materialized view with prefix as partition key (complex to maintain)");
    println!("   c) Application-level partitioning (store prefix separately)");
    println!("   d) Use clustering_store for tables that need fuzzy matching");
    
    println!("\n8. Current Approach:");
    println!("   - ALLOW FILTERING with full table scan");
    println!("   - This is the only reliable way for arbitrary fuzzy_bytes");
    println!("   - Performance impact depends on table size");
    
    // Test performance difference
    println!("\n9. Recommendation:");
    println!("   - For tables that frequently use get_leq:");
    println!("     * Convert to clustering_store if possible");
    println!("     * Design key structure to align fuzzy_bytes with clustering key");
    println!("   - For occasional get_leq on small tables:");
    println!("     * Current ALLOW FILTERING approach is acceptable");
    
    Ok(())
}