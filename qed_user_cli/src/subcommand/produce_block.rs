use plonky2::field::goldilocks_field::GoldilocksField;

use crate::rpc::provider::{QUserRpcProvider, RpcProvider};

use super::args::ProduceBlockArgs;
use anyhow::Result;

pub fn run(args: ProduceBlockArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config(args.rpc_config)?;
    provider.produce_block::<GoldilocksField>()?;

    Ok(())
}
