use plonky2::{
    field::{extension::Extendable, goldilocks_field::GoldilocksField},
    hash::hash_types::RichField,
};
use psy_data::qstore::imm::cmd_processor::PsyReadCommandProcessorSync;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{ups::state_reader::StateReader, vm::cfc_input::DapenContractFunctionCircuitInput};

type GF = GoldilocksField;
const D: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DPNSoftwareDefinedSignatureInput {
    pub cfc_input: DapenContractFunctionCircuitInput<GF>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plonky2SoftwareDefinedSignatureInput {
    pub state_reader_results: crate::ups::state_reader::StateReaderResults<GoldilocksField>,
    pub circuit_inputs: Vec<GoldilocksField>,
}
