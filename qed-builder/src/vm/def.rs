use serde::{Deserialize, Serialize};

use crate::{ops::DPNBuiltInDataType, OpType};

const INDEX_BITS: u64 = 32;
const INDEX_MASK: u64 = (1u64 << INDEX_BITS) - 1u64;

pub fn decode_indexed_op_id(id: u64) -> (DPNBuiltInDataType, usize) {
    (
        DPNBuiltInDataType::from(id >> INDEX_BITS),
        (id & INDEX_MASK) as usize,
    )
}
pub fn encode_indexed_op_id(data_type: DPNBuiltInDataType, index: usize) -> u64 {
    ((data_type as u64) << INDEX_BITS) | (index as u64)
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct DPNIndexedVarDef {
    pub data_type: DPNBuiltInDataType,
    pub index: usize,
    pub op_type: OpType,
    pub inputs: Vec<u64>,
}
impl DPNIndexedVarDef {
    pub fn get_combined_data_type_index(&self) -> u64 {
        encode_indexed_op_id(self.data_type, self.index)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct DPNAssertEqInfoIndexed {
    pub left: u64,
    pub right: u64,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct DPNFunctionCircuitDefinition {
    pub name: String,
    pub circuit_inputs: Vec<u64>,
    pub circuit_outputs: Vec<u64>,
    //   pub state_commands: Vec<DPNStateCmd<u64>>,
    //   pub state_command_resolution_indices: Vec<usize>,
    pub assertions: Vec<DPNAssertEqInfoIndexed>,
    pub definitions: Vec<DPNIndexedVarDef>,
}
