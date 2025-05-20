use ts_rs::TS;
use plonky2::field::goldilocks_field::GoldilocksField;

use crate::rpc::request::QUserRegistrationTreeMerkleProofFRPCRequest;

pub fn ts_export() -> anyhow::Result<()> {
    QUserRegistrationTreeMerkleProofFRPCRequest::<GoldilocksField>::export_all()?;
    Ok(())
}