use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQBinaryStoreAsync, KVQSerializable};
use qed_store::store::lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use psy_data::config::store_config::*;
use psy_data::models::kvq_merkle::key::KVQMerkleNodeKey;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_tree_clustering_correct() -> Result<()> {
    println!("=== Testing Tree Tables with Correct Key Structure ===\n");

    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_tree_correct");

    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 1)?;

    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_tree_correct").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };

    println!("1. Understanding the tree key structure:");
    println!("   KVQMerkleNodeKey fields:");
    println!("   - tree_id: 1 byte");
    println!("   - primary_id: 8 bytes");
    println!("   - secondary_id: 4 bytes");
    println!("   - level: 1 byte");
    println!("   - index: 8 bytes");
    println!("   - checkpoint_id: 8 bytes");
    println!("   Total serialized: 32 bytes (including 2-byte table type prefix)");

    // Create proper KVQMerkleNodeKey instances
    let tree_id = 1u8;
    let primary_id = 0u64;
    let secondary_id = 0u32;
    let level = 0u8;

    println!("\n2. Inserting test data:");
    let checkpoints = vec![100u64, 200, 300, 400, 500];

    for (i, checkpoint_id) in checkpoints.iter().enumerate() {
        let key = KVQMerkleNodeKey::<CHECKPOINT_TREE_TABLE_TYPE> {
            tree_id,
            primary_id,
            secondary_id,
            level,
            index: i as u64,
            checkpoint_id: *checkpoint_id,
        };

        let key_bytes = key.to_bytes()?;
        let value = format!("node_checkpoint_{}", checkpoint_id).into_bytes();

        mdbx_store.set_ref(&key_bytes, &value)?;
        <ScyllaStore as KVQBinaryStoreAsync>::set_ref(&scylla_store, &key_bytes, &value).await?;

        println!("   Checkpoint {} - key bytes: {:?} (len={})",
                 checkpoint_id, key_bytes, key_bytes.len());
    }

    println!("\n3. Testing get_leq with fuzzy_bytes = 8 (checkpoint_id size):");

    // Query for checkpoint 250
    let query_key = KVQMerkleNodeKey::<CHECKPOINT_TREE_TABLE_TYPE> {
        tree_id,
        primary_id,
        secondary_id,
        level,
        index: 2, // Same index as checkpoint 300
        checkpoint_id: 250,
    };

    let query_bytes = query_key.to_bytes()?;

    let mdbx_result = mdbx_store.get_leq(&query_bytes, 8)?;
    let scylla_result = <ScyllaStore as KVQBinaryStoreAsync>::get_leq(&scylla_store, &query_bytes, 8).await?;

    println!("   Query for checkpoint 250:");
    println!("   - LibMDBX: {:?}", mdbx_result.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_result.as_ref().map(|v| String::from_utf8_lossy(v)));

    // With fuzzy_bytes = 8, we're looking for keys with:
    // - Same prefix (first 24 bytes: table_type + tree location data)
    // - checkpoint_id <= 250
    // Expected: node_checkpoint_200

    println!("\n4. ScyllaDB partitioning with clustering_key_size = 8:");
    println!("   - Partition key: All tree location data (22 bytes)");
    println!("   - Clustering key: checkpoint_id (8 bytes)");
    println!("   - This design allows efficient queries for historical versions of the same node");

    // Test exact match
    println!("\n5. Testing exact match:");
    let exact_key = KVQMerkleNodeKey::<CHECKPOINT_TREE_TABLE_TYPE> {
        tree_id,
        primary_id,
        secondary_id,
        level,
        index: 2,
        checkpoint_id: 300,
    };

    let exact_bytes = exact_key.to_bytes()?;

    let mdbx_exact = mdbx_store.get_exact_if_exists(&exact_bytes)?;
    let scylla_exact = <ScyllaStore as KVQBinaryStoreAsync>::get_exact_if_exists(&scylla_store, &exact_bytes).await?;

    println!("   - LibMDBX: {:?}", mdbx_exact.as_ref().map(|v| String::from_utf8_lossy(v)));
    println!("   - ScyllaDB: {:?}", scylla_exact.as_ref().map(|v| String::from_utf8_lossy(v)));

    assert_eq!(mdbx_exact, scylla_exact);

    Ok(())
}
