use plonky2::field::goldilocks_field::GoldilocksField;

use crate::rpc::provider::{QUserRpcProvider, RpcProvider};

use super::args::ProduceBlockArgs;
use anyhow::Result;

pub async fn run(args: ProduceBlockArgs) -> Result<()> {
    let provider = RpcProvider::new(&args.rpc_address);
    provider.produce_block::<GoldilocksField>().await?;

    Ok(())
}
