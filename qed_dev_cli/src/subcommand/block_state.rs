use qed_prover::api::provider::RpcProvider;
use anyhow::{Ok, Result};
use qed_crypto::hash::traits::qhashable::QFieldHashable;
use qed_data::{
    config::store_config::QEDHasher, traits::qdatastore::qmetadata::QMetaDataStoreReaderSync,
};

pub fn get_latest_block_state(args: super::LatestBlockStateArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    let latest_block_state = provider.get_latest_l2_block_state()?;

    println!(
        "Latest block state: {}",
        serde_json::to_string_pretty(&latest_block_state)?
    );
    Ok(())
}

pub fn get_l2_block_state(args: super::BlockStateArgs) -> Result<()> {
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