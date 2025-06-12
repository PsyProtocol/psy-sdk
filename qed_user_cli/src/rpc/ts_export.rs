use ts_rs::TS;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::HashOut;
use serde::{Deserialize, Serialize};
use qed_core::data::qhashout::QHashOut;
use crate::rpc::request::QUserRegistrationTreeMerkleProofFRPCRequest;

pub fn ts_export() -> anyhow::Result<()> {
    HashOut::<GoldilocksField>::export_all()?;
    QHashOut::<GoldilocksField>::export_all()?;
    QUserRegistrationTreeMerkleProofFRPCRequest::<GoldilocksField>::export_all()?;
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, TS)]
#[serde(bound = "")]
#[ts(export,rename = "HashOut")]
pub struct HashOutRef {
    pub elements: [GoldilocksField; 4],
}