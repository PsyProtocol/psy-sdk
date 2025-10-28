use psy_vm::vm::cfc_input::DapenContractFunctionCircuitInput;
use psy_vm::dpn::vm::def::DPNFunctionCircuitDefinition;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use plonky2::field::goldilocks_field::GoldilocksField;

type GF = GoldilocksField;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct QSoftwareDefinedSignatureWitnessInput {
    pub cfc_input: DapenContractFunctionCircuitInput<GF>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct QSoftwareDefinedSignatureInput {
    pub fn_def: DPNFunctionCircuitDefinition,
    pub contract_id: u64,
    pub contract_state_tree_height: u8,
    pub session_proof_tree_height: u8,
    pub force_four_align: bool,
}

// Basic enum variants for SDK - prover can extend these with PLONKY2 variants
#[derive(Debug)]
pub enum SoftwareDefinedSignatureWitnessInput {
    Psy(QSoftwareDefinedSignatureWitnessInput),
}

#[derive(Debug)]
pub enum SoftwareDefinedSignatureInput {
    Psy(QSoftwareDefinedSignatureInput),
}