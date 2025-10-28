use std::str::FromStr;

use anyhow::Result;
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_core::data::qhashout::QHashOut;
use psy_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use psy_rust_sdk::provider::RpcProvider;

use crate::subcommand::args::CheckTxArgs;

type F = GoldilocksField;
pub async fn run(args: CheckTxArgs) -> Result<()> {
    tracing::info!("check tx: {}", serde_json::to_string_pretty(&args)?);
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;

    let checkpoint_id = match args.checkpoint_id {
        Some(id) => id,
        None => provider.get_latest_block_state().await?.checkpoint_id,
    };

    let tx_hash = QHashOut::<F>::from_str(&args.tx_hash)?;

    let is_onchain = provider.check_tx_is_confirmed(checkpoint_id, args.user_id, tx_hash).await?;
    tracing::info!("user {} tx {} is onchain: {}", args.user_id, args.tx_hash, is_onchain);

    Ok(())
}
