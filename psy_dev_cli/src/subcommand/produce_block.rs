use anyhow::Result;
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_rust_sdk::provider::{QUserRpcProvider, RpcProvider};

pub async fn run(args: super::ProduceBlockArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    provider.produce_block::<GoldilocksField>().await?;
    Ok(())
}
