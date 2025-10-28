use plonky2::field::goldilocks_field::GoldilocksField;
use psy_prover::local::provider::{QUserRpcProvider, RpcProvider};
use anyhow::Result;

pub async fn run(args: super::ProduceBlockArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    provider.produce_block::<GoldilocksField>().await?;
    Ok(())
}