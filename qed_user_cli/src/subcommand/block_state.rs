use super::args::{BlockStateArgs, LatestBlockStateArgs, UserIdArgs, UserLeafArgs};
use qed_prover::api::provider::RpcProvider;
use anyhow::{Ok, Result};
use qed_crypto::hash::traits::qhashable::QFieldHashable;
use qed_data::{
    config::store_config::QEDHasher, traits::qdatastore::qmetadata::QMetaDataStoreReaderSync,
};

pub fn get_lastest_block_state(args: LatestBlockStateArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    let latest_block_state = provider.get_latest_l2_block_state()?;

    println!(
        "Latest block state: {}",
        serde_json::to_string_pretty(&latest_block_state)?
    );
    Ok(())
}

pub fn get_l2_block_state(args: BlockStateArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    let block_state = provider.get_l2_block_state(args.checkpoint_id)?;
    let latest_checkpoint_id = provider.get_latest_l2_block_state()?.checkpoint_id;

    println!(
        " Block state {} -> {}: {}",
        args.checkpoint_id,
        latest_checkpoint_id,
        serde_json::to_string_pretty(&block_state)?
    );
    Ok(())
}

pub fn get_user_id(args: UserIdArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    let user_id = provider.get_user_id(args.pub_key)?;

    println!("user_id: {}", user_id);
    Ok(())
}

pub fn get_user_leaf(args: UserLeafArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    let user_id = provider.get_user_id(args.pub_key)?;

    let user_leaf_data = provider.get_user_leaf_data(args.checkpoint_id, user_id)?;

    println!(
        "user_leaf_data: {}",
        serde_json::to_string_pretty(&user_leaf_data)?
    );
    println!(
        "user_leaf_hash: {}",
        user_leaf_data.qfhash::<QEDHasher>().to_string()
    );

    Ok(())
}
