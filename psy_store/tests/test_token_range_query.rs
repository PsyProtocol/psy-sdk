use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync};
use psy_data::config::store_config::*;
use psy_store::store::{scylla::ScyllaStore, KVQlibmdbxStore};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_token_range_query() -> Result<()> {
    println!("=== Testing Token-Based Range Query Optimization ===\n");

    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_token_range");

    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;

    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_token_range").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };

    println!("1. Understanding token-based optimization potential:");
    println!("   In ScyllaDB, we could potentially use:");
    println!("   - WHERE token(partition_key) >= token(?) AND token(partition_key) <= token(?)");
    println!("   - This allows efficient range scans without ALLOW FILTERING");

    println!("\n2. Challenge with fuzzy matching:");
    println!("   - Fuzzy matching requires byte-wise comparison of the full key");
    println!("   - Token order != lexicographic order");
    println!("   - token(0x01) might be > token(0xFF)");

    println!("\n3. Potential optimization for specific cases:");
    println!("   When fuzzy_bytes aligns with key boundaries:");
    println!("   - If fuzzy_bytes matches partition key suffix");
    println!("   - We could construct a token range that covers all possible matches");

    // Example: checkpoint block states
    let table_type = CHECKPOINT_BLOCK_STATE_TABLE_TYPE;

    // Insert some test data
    for checkpoint_id in [100u64, 200, 300, 400, 500] {
        let mut key = table_type.to_be_bytes().to_vec();
        key.extend_from_slice(&checkpoint_id.to_be_bytes());

        let value = format!("checkpoint_{}", checkpoint_id).into_bytes();

        <KVQlibmdbxStore as KVQBinaryStore>::set_ref(&mdbx_store, &key, &value)?;
        <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla_store, &key, &value).await?;
    }

    println!("\n4. For checkpoint block states:");
    println!("   - Partition key = table_type (2 bytes)");
    println!("   - Clustering key = checkpoint_id (8 bytes)");
    println!("   - With fuzzy_bytes = 8, we stay within one partition");
    println!("   - No need for token range optimization");

    println!("\n5. For tree tables with fuzzy_bytes crossing boundaries:");
    println!("   - Would need to compute all possible partition keys");
    println!("   - Then use: WHERE token(partition_key) IN (token(?), token(?), ...)");
    println!("   - Or use multiple queries with token ranges");

    Ok(())
}
