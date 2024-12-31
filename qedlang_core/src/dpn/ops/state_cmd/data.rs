use std::hash::Hash;

use serde::{Deserialize, Serialize};


use super::types::{DPNStateCmdCore, DPNStateCommandType};



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdSetContractStateSlotHash<T> {
    pub condition: T,
    pub slot_index: T,
    pub value: [T; 4],
}

impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdSetContractStateSlotHash<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![
            self.condition,
            self.slot_index,
            self.value[0],
            self.value[1],
            self.value[2],
            self.value[3],
        ]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::SetContractStateSlotHash
    }

    fn get_output_felt_size(&self) -> usize {
        8
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdSetContractStateSlotSingle<T> {
    pub condition: T,
    pub sub_slot_index: T,
    pub value: T,
}

impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdSetContractStateSlotSingle<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![
            self.sub_slot_index,
            self.value
        ]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::SetContractStateSlotSingle
    }

    fn get_output_felt_size(&self) -> usize {
        2
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq)]
pub struct DPNStateCmdSetContractStateSlotRange<T> {
    pub condition: T,
    pub sub_slot_index: T,
    pub value: Vec<T>,
}

impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdSetContractStateSlotRange<T> {
    fn get_inputs(&self) -> Vec<T> {
        let mut base = Vec::with_capacity(self.value.len()+2);
        base.push(self.condition);
        base.push(self.sub_slot_index);
        base.extend(self.value.iter());
        base
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::SetContractStateSlotRange
    }

    fn get_output_felt_size(&self) -> usize {
        self.value.len()*2
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq)]
pub struct DPNStateCmdInvokeExternalContractFunction<T> {
    pub condition: T,
    pub contract_id: T,
    pub method_id: T,
    pub input_args: Vec<T>,
    pub num_outputs: u32,
}

impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdInvokeExternalContractFunction<T> {
    fn get_inputs(&self) -> Vec<T> {
        let mut base = Vec::with_capacity(self.input_args.len()+3);
        base.push(self.condition);
        base.push(self.contract_id);
        base.push(self.method_id);
        base.extend(self.input_args.iter());
        base
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::InvokeExternalContractFunction
    }

    fn get_output_felt_size(&self) -> usize {
        self.num_outputs as usize
    }
}




#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdGetSelfUserCurrentContractStateSlotHash<T> {
    pub slot_index: T,
}
impl<T> DPNStateCmdGetSelfUserCurrentContractStateSlotHash<T> {
    pub fn new(slot_index: T) -> Self {
        Self {
            slot_index
        }
    }
}
impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdGetSelfUserCurrentContractStateSlotHash<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.slot_index]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::GetSelfUserCurrentContractStateSlotHash
    }

    fn get_output_felt_size(&self) -> usize {
        4
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdGetSelfUserCurrentContractStateSlotSingle<T> {
    pub sub_slot_index: T,
}
impl<T> DPNStateCmdGetSelfUserCurrentContractStateSlotSingle<T> {
    pub fn new(sub_slot_index: T) -> Self {
        Self {
            sub_slot_index
        }
    }
}
impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdGetSelfUserCurrentContractStateSlotSingle<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.sub_slot_index]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::GetSelfUserCurrentContractStateSlotSingle
    }

    fn get_output_felt_size(&self) -> usize {
        1
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdGetSelfUserCurrentContractStateSlotRange<T> {
    pub sub_slot_index: T,
    pub length: u32,
}
impl<T> DPNStateCmdGetSelfUserCurrentContractStateSlotRange<T> {
    pub fn new(sub_slot_index: T, length: u32) -> Self {
        Self {
            sub_slot_index,
            length,
        }
    }
}
impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdGetSelfUserCurrentContractStateSlotRange<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.sub_slot_index]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::GetSelfUserCurrentContractStateSlotRange
    }

    fn get_output_felt_size(&self) -> usize {
        self.length as usize
    }
}



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdGetSelfUserExternalContractStateSlotHash<T> {
    pub contract_id: T,
    pub slot_index: T,
    pub contract_state_tree_height: u8,
}
impl<T> DPNStateCmdGetSelfUserExternalContractStateSlotHash<T> {
    pub fn new(contract_id: T, contract_state_tree_height: u8, slot_index: T) -> Self {
        Self {
            contract_id,
            contract_state_tree_height,
            slot_index
        }
    }
}
impl<T: Ord + Hash + Clone + Copy> DPNStateCmdCore<T> for DPNStateCmdGetSelfUserExternalContractStateSlotHash<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.contract_id, self.slot_index]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::GetSelfUserExternalContractStateSlotHash
    }

    fn get_output_felt_size(&self) -> usize {
        4
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdGetSelfUserExternalContractStateSlotSingle<T> {
    pub contract_id: T,
    pub sub_slot_index: T,
    pub contract_state_tree_height: u8,
}
impl<T> DPNStateCmdGetSelfUserExternalContractStateSlotSingle<T> {
    pub fn new(contract_id: T, contract_state_tree_height: u8, sub_slot_index: T) -> Self {
        Self {
            contract_id,
            contract_state_tree_height,
            sub_slot_index
        }
    }
}
impl<T: Ord + Hash + Clone + Copy> DPNStateCmdCore<T> for DPNStateCmdGetSelfUserExternalContractStateSlotSingle<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.contract_id, self.sub_slot_index]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::GetSelfUserExternalContractStateSlotSingle
    }

    fn get_output_felt_size(&self) -> usize {
        1
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdGetSelfUserExternalContractStateSlotRange<T> {
    pub contract_id: T,
    pub sub_slot_index: T,
    pub length: u32,
    pub contract_state_tree_height: u8,
}
impl<T> DPNStateCmdGetSelfUserExternalContractStateSlotRange<T> {
    pub fn new(contract_id: T, contract_state_tree_height: u8, sub_slot_index: T, length: u32) -> Self {
        Self {
            contract_id,
            contract_state_tree_height,
            sub_slot_index,
            length,
        }
    }
}
impl<T: Ord + Hash + Clone + Copy> DPNStateCmdCore<T> for DPNStateCmdGetSelfUserExternalContractStateSlotRange<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.contract_id, self.sub_slot_index]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::GetSelfUserExternalContractStateSlotRange
    }

    fn get_output_felt_size(&self) -> usize {
        self.length as usize
    }
}



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdGetOtherUserContractStateSlotHash<T> {
    pub user_id: T,
    pub contract_id: T,
    pub slot_index: T,
    pub contract_state_tree_height: u8,
}
impl<T> DPNStateCmdGetOtherUserContractStateSlotHash<T> {
    pub fn new(user_id: T, contract_id: T, contract_state_tree_height: u8, slot_index: T) -> Self {
        Self {
            user_id,
            contract_id,
            contract_state_tree_height,
            slot_index
        }
    }
}
impl<T: Ord + Hash + Clone + Copy> DPNStateCmdCore<T> for DPNStateCmdGetOtherUserContractStateSlotHash<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.user_id, self.contract_id, self.slot_index]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::GetOtherUserContractStateSlotHash
    }

    fn get_output_felt_size(&self) -> usize {
        4
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdGetOtherUserContractStateSlotSingle<T> {
    pub user_id: T,
    pub contract_id: T,
    pub sub_slot_index: T,
    pub contract_state_tree_height: u8,
}
impl<T> DPNStateCmdGetOtherUserContractStateSlotSingle<T> {
    pub fn new(user_id: T, contract_id: T, contract_state_tree_height: u8, sub_slot_index: T) -> Self {
        Self {
            user_id,
            contract_id,
            contract_state_tree_height,
            sub_slot_index
        }
    }
}
impl<T: Ord + Hash + Clone + Copy> DPNStateCmdCore<T> for DPNStateCmdGetOtherUserContractStateSlotSingle<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.user_id, self.contract_id, self.sub_slot_index]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::GetOtherUserContractStateSlotSingle
    }

    fn get_output_felt_size(&self) -> usize {
        1
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdGetOtherUserContractStateSlotRange<T> {
    pub user_id: T,
    pub contract_id: T,
    pub sub_slot_index: T,
    pub length: u32,
    pub contract_state_tree_height: u8,
}
impl<T> DPNStateCmdGetOtherUserContractStateSlotRange<T> {
    pub fn new(user_id: T, contract_id: T, contract_state_tree_height: u8, sub_slot_index: T, length: u32) -> Self {
        Self {
            user_id,
            contract_id,
            contract_state_tree_height,
            sub_slot_index,
            length,
        }
    }
}
impl<T: Ord + Hash + Clone + Copy> DPNStateCmdCore<T> for DPNStateCmdGetOtherUserContractStateSlotRange<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.user_id, self.contract_id, self.sub_slot_index]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::GetOtherUserContractStateSlotRange
    }

    fn get_output_felt_size(&self) -> usize {
        self.length as usize
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq)]
#[serde(tag = "type")]
pub enum DPNStateCmd<T> {
    SetContractStateSlotHash(DPNStateCmdSetContractStateSlotHash<T>),
    SetContractStateSlotSingle(DPNStateCmdSetContractStateSlotSingle<T>),
    SetContractStateSlotRange(DPNStateCmdSetContractStateSlotRange<T>),
    InvokeExternalContractFunction(DPNStateCmdInvokeExternalContractFunction<T>),
    GetSelfUserCurrentContractStateSlotHash(DPNStateCmdGetSelfUserCurrentContractStateSlotHash<T>),
    GetSelfUserCurrentContractStateSlotSingle(DPNStateCmdGetSelfUserCurrentContractStateSlotSingle<T>),
    GetSelfUserCurrentContractStateSlotRange(DPNStateCmdGetSelfUserCurrentContractStateSlotRange<T>),
    GetSelfUserExternalContractStateSlotHash(DPNStateCmdGetSelfUserExternalContractStateSlotHash<T>),
    GetSelfUserExternalContractStateSlotSingle(DPNStateCmdGetSelfUserExternalContractStateSlotSingle<T>),
    GetSelfUserExternalContractStateSlotRange(DPNStateCmdGetSelfUserExternalContractStateSlotRange<T>),
    GetOtherUserContractStateSlotHash(DPNStateCmdGetOtherUserContractStateSlotHash<T>),
    GetOtherUserContractStateSlotSingle(DPNStateCmdGetOtherUserContractStateSlotSingle<T>),
    GetOtherUserContractStateSlotRange(DPNStateCmdGetOtherUserContractStateSlotRange<T>),
}
impl<T> DPNStateCmd<T> {
    pub fn set_contract_state_slot_hash(condition: T, slot_index: T, value: [T; 4]) -> Self {
        DPNStateCmd::SetContractStateSlotHash(DPNStateCmdSetContractStateSlotHash {
            condition,
            slot_index,
            value
        })
    }
    pub fn set_contract_state_slot_single(condition: T, sub_slot_index: T, value: T) -> Self {
        DPNStateCmd::SetContractStateSlotSingle(DPNStateCmdSetContractStateSlotSingle {
            condition,
            sub_slot_index,
            value
        })
    }
    pub fn set_contract_state_slot_range(condition: T,sub_slot_index: T, value: Vec<T>) -> Self {
        DPNStateCmd::SetContractStateSlotRange(DPNStateCmdSetContractStateSlotRange {
            condition,
            sub_slot_index,
            value
        })
    }
    pub fn invoke_external_contract_function(condition: T, contract_id: T, method_id: T, input_args: Vec<T>, num_outputs: u32) -> Self {
        DPNStateCmd::InvokeExternalContractFunction(DPNStateCmdInvokeExternalContractFunction {
            condition,
            contract_id,
            method_id,
            input_args,
            num_outputs
        })
    }
    pub fn get_self_user_current_contract_state_slot_hash(slot_index: T) -> Self {
        DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(DPNStateCmdGetSelfUserCurrentContractStateSlotHash::<T>::new(slot_index))
    }
    pub fn get_self_user_current_contract_state_slot_single(sub_slot_index: T) -> Self {
        DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(DPNStateCmdGetSelfUserCurrentContractStateSlotSingle::<T>::new(sub_slot_index))
    }
    pub fn get_self_user_current_contract_state_slot_range(sub_slot_index: T, length: u32) -> Self {
        DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(DPNStateCmdGetSelfUserCurrentContractStateSlotRange::<T>::new(sub_slot_index, length))
    }
    pub fn get_self_user_external_contract_state_slot_hash(contract_id: T, contract_state_tree_height: u8, slot_index: T) -> Self {
        DPNStateCmd::GetSelfUserExternalContractStateSlotHash(DPNStateCmdGetSelfUserExternalContractStateSlotHash::<T>::new(contract_id, contract_state_tree_height, slot_index))
    }
    pub fn get_self_user_external_contract_state_slot_single(contract_id: T, contract_state_tree_height: u8, sub_slot_index: T) -> Self {
        DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(DPNStateCmdGetSelfUserExternalContractStateSlotSingle::<T>::new(contract_id, contract_state_tree_height, sub_slot_index))
    }
    pub fn get_self_user_external_contract_state_slot_range(contract_id: T, contract_state_tree_height: u8, sub_slot_index: T, length: u32) -> Self {
        DPNStateCmd::GetSelfUserExternalContractStateSlotRange(DPNStateCmdGetSelfUserExternalContractStateSlotRange::<T>::new(contract_id, contract_state_tree_height, sub_slot_index, length))
    }
    pub fn get_other_user_contract_state_slot_hash(user_id: T, contract_id: T, contract_state_tree_height: u8, slot_index: T) -> Self {
        DPNStateCmd::GetOtherUserContractStateSlotHash(DPNStateCmdGetOtherUserContractStateSlotHash::<T>::new(user_id, contract_id, contract_state_tree_height, slot_index))
    }
    pub fn get_other_user_contract_state_slot_single(user_id: T, contract_id: T, contract_state_tree_height: u8, sub_slot_index: T) -> Self {
        DPNStateCmd::GetOtherUserContractStateSlotSingle(DPNStateCmdGetOtherUserContractStateSlotSingle::<T>::new(user_id, contract_id, contract_state_tree_height, sub_slot_index))
    }
    pub fn get_other_user_contract_state_slot_range(user_id: T, contract_id: T, contract_state_tree_height: u8, sub_slot_index: T, length: u32) -> Self {
        DPNStateCmd::GetOtherUserContractStateSlotRange(DPNStateCmdGetOtherUserContractStateSlotRange::<T>::new(user_id, contract_id, contract_state_tree_height, sub_slot_index, length))
    }
    
}
impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmd<T> {
    fn get_inputs(&self) -> Vec<T> {
        match self {
            DPNStateCmd::SetContractStateSlotHash(c) => c.get_inputs(),
            DPNStateCmd::SetContractStateSlotSingle(c) => c.get_inputs(),
            DPNStateCmd::SetContractStateSlotRange(c) => c.get_inputs(),
            DPNStateCmd::InvokeExternalContractFunction(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(c) => c.get_inputs(),
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => c.get_inputs(),
            DPNStateCmd::GetOtherUserContractStateSlotSingle(c) => c.get_inputs(),
            DPNStateCmd::GetOtherUserContractStateSlotRange(c) => c.get_inputs(),
        }
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        match self {
            DPNStateCmd::SetContractStateSlotHash(c) => c.get_state_command_type(),
            DPNStateCmd::SetContractStateSlotSingle(c) => c.get_state_command_type(),
            DPNStateCmd::SetContractStateSlotRange(c) => c.get_state_command_type(),
            DPNStateCmd::InvokeExternalContractFunction(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(c) => c.get_state_command_type(),
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => c.get_state_command_type(),
            DPNStateCmd::GetOtherUserContractStateSlotSingle(c) => c.get_state_command_type(),
            DPNStateCmd::GetOtherUserContractStateSlotRange(c) => c.get_state_command_type(),
        }
    }

    fn get_output_felt_size(&self) -> usize {
        match self {
            DPNStateCmd::SetContractStateSlotHash(c) => c.get_output_felt_size(),
            DPNStateCmd::SetContractStateSlotSingle(c) => c.get_output_felt_size(),
            DPNStateCmd::SetContractStateSlotRange(c) => c.get_output_felt_size(),
            DPNStateCmd::InvokeExternalContractFunction(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(c) => c.get_output_felt_size(),
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => c.get_output_felt_size(),
            DPNStateCmd::GetOtherUserContractStateSlotSingle(c) => c.get_output_felt_size(),
            DPNStateCmd::GetOtherUserContractStateSlotRange(c) => c.get_output_felt_size(),
        }
    }
}


impl<T: Copy + Clone + Hash + Ord> DPNStateCmd<T> {

    pub fn convert_to_u64(&self, inputs_as_u64: &[u64]) -> DPNStateCmd<u64> {
        match self {
            DPNStateCmd::SetContractStateSlotHash(c) => {
                DPNStateCmd::SetContractStateSlotHash(DPNStateCmdSetContractStateSlotHash {
                    condition: inputs_as_u64[0],
                    slot_index: inputs_as_u64[1],
                    value: [
                        inputs_as_u64[2],
                        inputs_as_u64[3],
                        inputs_as_u64[4],
                        inputs_as_u64[5],
                    ],
                })
            },
            DPNStateCmd::SetContractStateSlotSingle(c) => {
                DPNStateCmd::SetContractStateSlotSingle(DPNStateCmdSetContractStateSlotSingle {
                    condition: inputs_as_u64[0],
                    sub_slot_index: inputs_as_u64[1],
                    value: inputs_as_u64[2],
                })
            },
            DPNStateCmd::SetContractStateSlotRange(c) => {
                DPNStateCmd::SetContractStateSlotRange(DPNStateCmdSetContractStateSlotRange {
                    condition: inputs_as_u64[0],
                    sub_slot_index: inputs_as_u64[1],
                    value: inputs_as_u64[2..].to_vec(),
                })
            },
            DPNStateCmd::InvokeExternalContractFunction(c) => {
                DPNStateCmd::InvokeExternalContractFunction(DPNStateCmdInvokeExternalContractFunction {
                    condition: inputs_as_u64[0],
                    contract_id: inputs_as_u64[1],
                    method_id: inputs_as_u64[2],
                    input_args: inputs_as_u64[3..].to_vec(),
                    num_outputs: c.num_outputs,
                })
            },
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(c) => {
                DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(DPNStateCmdGetSelfUserCurrentContractStateSlotHash::<u64>::new(inputs_as_u64[0]))
            },
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(c) => {
                DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(DPNStateCmdGetSelfUserCurrentContractStateSlotSingle::<u64>::new(inputs_as_u64[0]))
            },
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(c) => {
                DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(DPNStateCmdGetSelfUserCurrentContractStateSlotRange::<u64>::new(inputs_as_u64[0], c.length))
            },
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => {
                DPNStateCmd::GetSelfUserExternalContractStateSlotHash(DPNStateCmdGetSelfUserExternalContractStateSlotHash::<u64>::new(inputs_as_u64[0], c.contract_state_tree_height, inputs_as_u64[1]))
            },
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(c) => {
                DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(DPNStateCmdGetSelfUserExternalContractStateSlotSingle::<u64>::new(inputs_as_u64[0], c.contract_state_tree_height, inputs_as_u64[1]))
            },
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(c) => {
                DPNStateCmd::GetSelfUserExternalContractStateSlotRange(DPNStateCmdGetSelfUserExternalContractStateSlotRange::<u64>::new(inputs_as_u64[0], c.contract_state_tree_height, inputs_as_u64[1], c.length))
            },
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => {
                DPNStateCmd::GetOtherUserContractStateSlotHash(DPNStateCmdGetOtherUserContractStateSlotHash::<u64>::new(inputs_as_u64[0], inputs_as_u64[1], c.contract_state_tree_height, inputs_as_u64[2]))
            },
            DPNStateCmd::GetOtherUserContractStateSlotSingle(c) => {
                DPNStateCmd::GetOtherUserContractStateSlotSingle(DPNStateCmdGetOtherUserContractStateSlotSingle::<u64>::new(inputs_as_u64[0], inputs_as_u64[1], c.contract_state_tree_height, inputs_as_u64[2]))
            },
            DPNStateCmd::GetOtherUserContractStateSlotRange(c) => {
                DPNStateCmd::GetOtherUserContractStateSlotRange(DPNStateCmdGetOtherUserContractStateSlotRange::<u64>::new(inputs_as_u64[0], inputs_as_u64[1], c.contract_state_tree_height, inputs_as_u64[2], c.length))
            },
        }
    } 

}