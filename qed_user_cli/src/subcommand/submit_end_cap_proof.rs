use std::fs;

use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use qed_data::guta::end_cap_input::SubmitUserEndCapNonProofInput;

use crate::rpc::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QSubmitEndCapRPCRequest,
};

use super::args::SubmitEndCapArgs;
use anyhow::Result;

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

pub async fn run(args: SubmitEndCapArgs) -> Result<()> {
    let provider = RpcProvider::new(&args.rpc_address);

    let user_ec_input: SubmitUserEndCapNonProofInput<F> =
        serde_json::from_str(&fs::read_to_string(args.user_ec_input_path)?)?;
    let proof: ProofWithPublicInputs<F, C, D> =
        serde_json::from_str(&fs::read_to_string(args.proof_path)?)?;

    provider
        .submit_end_cap_proof::<F>(QSubmitEndCapRPCRequest {
            user_ec_input,
            proof,
        })
        .await?;

    Ok(())
}
