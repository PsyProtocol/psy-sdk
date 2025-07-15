use plonky2::field::goldilocks_field::GoldilocksField;
use qed_prover::api::provider::{QUserRpcProvider, RpcProvider};
use anyhow::Result;

pub fn run(args: super::ProduceBlockArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    provider.produce_block::<GoldilocksField>()?;
    Ok(())
}