use kvq::{
    adapters::standard::KVQStandardAdapter,
    traits::{KVQBinaryStoreAsync, KVQStoreAdapterAsync, KVQStoreAdapterReaderAsync},
};
use psy_data::{
    config::store_config::CHECKPOINT_BLOCK_STATE_TABLE_TYPE,
    qdata::{checkpoint::PsyL2BlockState, u64_key::U64TableKey},
};
use psy_store::store::scylla::kvq_store::ScyllaKVQStore;

mod common;
use common::*;

#[tokio::test]
async fn test_checkpoint_block_state_get_latest() -> anyhow::Result<()> {
    let config = TestConfig::new().await?;

    let store = ScyllaKVQStore::new(&config.uri, &config.keyspace, "checkpoint_block_states").await?;

    // Create adapter for block state
    type BlockStateKey = U64TableKey<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>;
    type BlockStateAdapter = KVQStandardAdapter<ScyllaKVQStore, BlockStateKey, PsyL2BlockState>;

    // Insert some test block states
    let block_state_0 = PsyL2BlockState {
        checkpoint_id: 0,
        next_add_withdrawal_id: 0,
        next_process_withdrawal_id: 0,
        next_deposit_id: 0,
        total_deposits_claimed_epoch: 0,
        next_user_id: 0,
        end_balance: 0,
        next_contract_id: 0,
    };

    let block_state_1 = PsyL2BlockState {
        checkpoint_id: 1,
        next_add_withdrawal_id: 0,
        next_process_withdrawal_id: 0,
        next_deposit_id: 0,
        total_deposits_claimed_epoch: 0,
        next_user_id: 1,
        end_balance: 1000,
        next_contract_id: 0,
    };

    let block_state_5 = PsyL2BlockState {
        checkpoint_id: 5,
        next_add_withdrawal_id: 2,
        next_process_withdrawal_id: 1,
        next_deposit_id: 10,
        total_deposits_claimed_epoch: 0,
        next_user_id: 5,
        end_balance: 5000,
        next_contract_id: 1,
    };

    // Insert block states
    BlockStateAdapter::set_ref(&store, &U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(0), &block_state_0).await?;
    BlockStateAdapter::set_ref(&store, &U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(1), &block_state_1).await?;
    BlockStateAdapter::set_ref(&store, &U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(5), &block_state_5).await?;

    // Test get_leq for latest block state (simulating get_latest_block_state)
    let max_key = U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(0xffffffffffffff_u64);
    let fuzzy_bytes = 8; // CHECKPOINT_ID_FUZZY_SIZE

    let result = BlockStateAdapter::get_leq(&store, &max_key, fuzzy_bytes).await?;

    // Should return the block state with highest checkpoint_id (5)
    assert!(result.is_some(), "get_leq should return a result");
    let latest = result.unwrap();
    assert_eq!(latest.checkpoint_id, 5, "Should return the latest checkpoint");

    // Test with specific checkpoint IDs
    let result = BlockStateAdapter::get_leq(&store, &U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(3), fuzzy_bytes).await?;
    assert!(result.is_some());
    let block = result.unwrap();
    assert_eq!(block.checkpoint_id, 1, "Should return checkpoint 1 when looking for <= 3");

    // Test exact match
    let result = BlockStateAdapter::get_leq(&store, &U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(5), fuzzy_bytes).await?;
    assert!(result.is_some());
    let block = result.unwrap();
    assert_eq!(block.checkpoint_id, 5, "Should return checkpoint 5 when looking for <= 5");

    config.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_empty_checkpoint_block_state() -> anyhow::Result<()> {
    let config = TestConfig::new().await?;

    let store = ScyllaKVQStore::new(&config.uri, &config.keyspace, "checkpoint_block_states").await?;

    // Create adapter for block state
    type BlockStateKey = U64TableKey<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>;
    type BlockStateAdapter = KVQStandardAdapter<ScyllaKVQStore, BlockStateKey, PsyL2BlockState>;

    // Test get_leq on empty table
    let max_key = U64TableKey::<CHECKPOINT_BLOCK_STATE_TABLE_TYPE>(0xffffffffffffff_u64);
    let fuzzy_bytes = 8;

    let result = BlockStateAdapter::get_leq(&store, &max_key, fuzzy_bytes).await?;
    assert!(result.is_none(), "get_leq should return None on empty table");

    config.cleanup().await?;
    Ok(())
}
