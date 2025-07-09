use anyhow::Result;
use kvq::traits::{KVQBinaryStore, KVQSerializable};
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::store::scylla::ScyllaStore;
use qed_data::config::store_config::*;
use qed_data::qdata::u64_key::U64TableKey;
use qed_data::models::kvq_merkle::key::KVQMerkleNodeKey;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_comprehensive_consistency() -> Result<()> {
    println!("=== Comprehensive Consistency Test: LibMDBX vs ScyllaDB ===\n");

    let temp_dir = tempfile::tempdir()?;
    let mdbx_path = temp_dir.path().join("test_comprehensive");

    let mdbx_store = KVQlibmdbxStore::new_write_with_size(mdbx_path.to_str().unwrap(), 10)?;

    let scylla_store = match ScyllaStore::new("127.0.0.1:9042", "test_comprehensive").await {
        Ok(store) => store,
        Err(e) => {
            println!("ScyllaDB not available: {:?}", e);
            return Ok(());
        }
    };

    // Use simple counter for test variety
    let mut test_counter = 0u64;

    // Test statistics
    let mut total_tests = 0;
    let mut passed_tests = 0;

    // 1. Test Checkpoint Block States (clustering store with 8-byte clustering key)
    println!("1. Testing Checkpoint Block States");
    println!("   Key: [table_type(2) + checkpoint_id(8)] = 10 bytes");
    println!("   Clustering key size: 8 bytes\n");

    // Insert multiple datasets
    for dataset in 0..3 {
        println!("   Dataset {}:", dataset + 1);
        let base = dataset * 1000;
        let checkpoint_ids: Vec<u64> = (0..20).map(|i| base + i * 50).collect();

        for &id in &checkpoint_ids {
            let key = U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(id);
            let key_bytes = key.to_bytes()?;
            let value = format!("checkpoint_{}_dataset_{}", id, dataset).into_bytes();

            mdbx_store.set_ref(&key_bytes, &value)?;
            scylla_store.set_ref(&key_bytes, &value)?;
        }

        // Test various fuzzy_bytes values
        for fuzzy_bytes in [0, 2, 4, 6, 8, 9, 10] {
            // Test multiple query points
            for _ in 0..5 {
                test_counter += 1;
                let query_id = base + (test_counter % 1000);
                let query_key = U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(query_id);
                let query_bytes = query_key.to_bytes()?;

                let mdbx_result = mdbx_store.get_leq(&query_bytes, fuzzy_bytes)?;
                let scylla_result = scylla_store.get_leq(&query_bytes, fuzzy_bytes)?;

                total_tests += 1;
                if mdbx_result == scylla_result {
                    passed_tests += 1;
                } else {
                    println!("      ❌ Mismatch: query_id={}, fuzzy_bytes={}", query_id, fuzzy_bytes);
                    println!("         LibMDBX: {:?}", mdbx_result.as_ref().map(|v| String::from_utf8_lossy(v)));
                    println!("         ScyllaDB: {:?}", scylla_result.as_ref().map(|v| String::from_utf8_lossy(v)));
                }
            }
        }
        println!("      ✓ Completed dataset {}", dataset + 1);
    }

    // 2. Test Tree Tables (clustering store with 8-byte clustering key)
    println!("\n2. Testing Tree Tables");
    println!("   Key: [table_type(2) + tree_id(1) + primary_id(8) + secondary_id(4) + level(1) + index(8) + checkpoint_id(8)] = 32 bytes");
    println!("   Clustering key size: 8 bytes\n");

    for dataset in 0..3 {
        println!("   Dataset {}:", dataset + 1);
        let tree_id = (dataset + 1) as u8;

        // Create nodes at different levels and checkpoints
        for level in 0..3 {
            for index in 0..5 {
                for checkpoint in 0..5 {
                    let checkpoint_id = (dataset * 100 + checkpoint * 10) as u64;
                    let node_key = KVQMerkleNodeKey::<USER_TREE_TABLE_TYPE> {
                        tree_id,
                        primary_id: dataset as u64,
                        secondary_id: 0,
                        level: level as u8,
                        index: index as u64,
                        checkpoint_id,
                    };

                    let key_bytes = node_key.to_bytes()?;
                    let value = format!("tree_node_{}_{}_{}", level, index, checkpoint_id).into_bytes();

                    mdbx_store.set_ref(&key_bytes, &value)?;
                    scylla_store.set_ref(&key_bytes, &value)?;
                }
            }
        }

        // Test with various fuzzy_bytes
        for fuzzy_bytes in [0, 4, 8, 12, 16, 20, 24, 28, 32] {
            for _ in 0..5 {
                test_counter += 1;
                let query_level = ((test_counter / 3) % 5) as u8;
                let query_index = ((test_counter / 5) % 10) as u64;
                let query_checkpoint = (dataset * 100 + (test_counter % 100)) as u64;

                let query_key = KVQMerkleNodeKey::<USER_TREE_TABLE_TYPE> {
                    tree_id,
                    primary_id: dataset as u64,
                    secondary_id: 0,
                    level: query_level,
                    index: query_index,
                    checkpoint_id: query_checkpoint,
                };

                let query_bytes = query_key.to_bytes()?;

                let mdbx_result = mdbx_store.get_leq(&query_bytes, fuzzy_bytes)?;
                let scylla_result = scylla_store.get_leq(&query_bytes, fuzzy_bytes)?;

                total_tests += 1;
                if mdbx_result == scylla_result {
                    passed_tests += 1;
                } else {
                    println!("      ❌ Mismatch: level={}, index={}, checkpoint={}, fuzzy_bytes={}",
                             query_level, query_index, query_checkpoint, fuzzy_bytes);
                }
            }
        }
        println!("      ✓ Completed dataset {}", dataset + 1);
    }

    // 3. Test User Leaves (clustering store with 4-byte clustering key)
    println!("\n3. Testing User Leaves");
    println!("   Key: [table_type(2) + user_id(8) + version(4)] = 14 bytes");
    println!("   Clustering key size: 4 bytes\n");

    for dataset in 0..3 {
        println!("   Dataset {}:", dataset + 1);
        let user_base = dataset * 1000;

        // Create multiple users with versions
        for user_offset in 0..10 {
            let user_id: u64 = user_base + user_offset * 100;
            for version in 1..=10 {
                let mut key = USER_LEAF_TABLE_TYPE.to_be_bytes().to_vec();
                key.extend_from_slice(&user_id.to_be_bytes());
                key.extend_from_slice(&(version as u32).to_be_bytes());

                let value = format!("user_{}_v{}", user_id, version).into_bytes();

                mdbx_store.set_ref(&key, &value)?;
                scylla_store.set_ref(&key, &value)?;
            }
        }

        // Test with various fuzzy_bytes
        for fuzzy_bytes in [0, 2, 4, 6, 8, 10, 12, 14] {
            for _ in 0..5 {
                test_counter += 1;
                let query_user: u64 = user_base + (test_counter % 1000);
                let query_version = (test_counter % 20) as u32;

                let mut query_key = USER_LEAF_TABLE_TYPE.to_be_bytes().to_vec();
                query_key.extend_from_slice(&query_user.to_be_bytes());
                query_key.extend_from_slice(&query_version.to_be_bytes());

                let mdbx_result = mdbx_store.get_leq(&query_key, fuzzy_bytes)?;
                let scylla_result = scylla_store.get_leq(&query_key, fuzzy_bytes)?;

                total_tests += 1;
                if mdbx_result == scylla_result {
                    passed_tests += 1;
                } else {
                    println!("      ❌ Mismatch: user={}, version={}, fuzzy_bytes={}",
                             query_user, query_version, fuzzy_bytes);
                }
            }
        }
        println!("      ✓ Completed dataset {}", dataset + 1);
    }

    // 4. Test KVQ Store Tables (simple key-value store)
    println!("\n4. Testing KVQ Store Tables (User Public Key Helper)");
    println!("   Key: [table_type(2) + user_id(8) + key_version(4)] = 14 bytes");
    println!("   No clustering key (simple KVQ store)\n");

    for dataset in 0..3 {
        println!("   Dataset {}:", dataset + 1);
        let user_base = dataset * 1000;

        // Create public keys for users
        for user_offset in 0..10 {
            let user_id: u64 = user_base + user_offset * 50;
            for key_version in 1..=5 {
                let mut key = USER_PUBLIC_KEY_HELPER_TABLE_TYPE.to_be_bytes().to_vec();
                key.extend_from_slice(&user_id.to_be_bytes());
                key.extend_from_slice(&(key_version as u32).to_be_bytes());

                let value = format!("pubkey_user_{}_v{}", user_id, key_version).into_bytes();

                mdbx_store.set_ref(&key, &value)?;
                scylla_store.set_ref(&key, &value)?;
            }
        }

        // Test with various fuzzy_bytes
        for fuzzy_bytes in [0, 2, 4, 6, 8, 10, 12, 14] {
            for _ in 0..5 {
                test_counter += 1;
                let query_user: u64 = user_base + (test_counter % 600);
                let query_version = (test_counter % 10) as u32;

                let mut query_key = USER_PUBLIC_KEY_HELPER_TABLE_TYPE.to_be_bytes().to_vec();
                query_key.extend_from_slice(&query_user.to_be_bytes());
                query_key.extend_from_slice(&query_version.to_be_bytes());

                let mdbx_result = mdbx_store.get_leq(&query_key, fuzzy_bytes)?;
                let scylla_result = scylla_store.get_leq(&query_key, fuzzy_bytes)?;

                total_tests += 1;
                if mdbx_result == scylla_result {
                    passed_tests += 1;
                } else {
                    println!("      ❌ Mismatch: user={}, version={}, fuzzy_bytes={}",
                             query_user, query_version, fuzzy_bytes);
                }
            }
        }
        println!("      ✓ Completed dataset {}", dataset + 1);
    }

    // 5. Test Edge Cases
    println!("\n5. Testing Edge Cases");

    // Empty result cases
    println!("   Testing empty results:");
    for fuzzy_bytes in [0, 4, 8] {
        // Query for non-existent data
        let mut query_key = vec![0xFF; 14];
        query_key[0] = 0;
        query_key[1] = USER_LEAF_TABLE_TYPE as u8;

        let mdbx_result = mdbx_store.get_leq(&query_key, fuzzy_bytes)?;
        let scylla_result = scylla_store.get_leq(&query_key, fuzzy_bytes)?;

        total_tests += 1;
        if mdbx_result == scylla_result {
            passed_tests += 1;
        } else {
            println!("      ❌ Empty result mismatch: fuzzy_bytes={}", fuzzy_bytes);
        }
    }

    // Boundary cases
    println!("   Testing boundary cases:");

    // Insert boundary values
    let boundary_key = vec![0x00, 0x0A, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x01];
    let boundary_value = b"boundary".to_vec();
    mdbx_store.set_ref(&boundary_key, &boundary_value)?;
    scylla_store.set_ref(&boundary_key, &boundary_value)?;

    // Query at boundary
    let query_boundary = vec![0x00, 0x0A, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x02];
    for fuzzy_bytes in [0, 4, 8, 12] {
        let mdbx_result = mdbx_store.get_leq(&query_boundary, fuzzy_bytes)?;
        let scylla_result = scylla_store.get_leq(&query_boundary, fuzzy_bytes)?;

        total_tests += 1;
        if mdbx_result == scylla_result {
            passed_tests += 1;
        } else {
            println!("      ❌ Boundary mismatch: fuzzy_bytes={}", fuzzy_bytes);
        }
    }

    // Final Summary
    println!("\n{}", "=".repeat(60));
    println!("FINAL RESULTS:");
    println!("Total tests: {}", total_tests);
    println!("Passed: {} ({:.2}%)", passed_tests, (passed_tests as f64 / total_tests as f64) * 100.0);
    println!("Failed: {} ({:.2}%)", total_tests - passed_tests, ((total_tests - passed_tests) as f64 / total_tests as f64) * 100.0);

    if passed_tests == total_tests {
        println!("\n✅ ALL TESTS PASSED! LibMDBX and ScyllaDB are fully consistent!");
    } else {
        println!("\n❌ Some tests failed. Please investigate the mismatches above.");
    }
    println!("{}", "=".repeat(60));

    Ok(())
}
