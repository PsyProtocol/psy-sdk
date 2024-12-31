use serde::{Deserialize, Serialize};

use crate::dpn::ops::sym_felt::SymFeltRef;

use super::types::{DPNStateCmdCore, DPNStateCommandType};



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct DPNStateCmdSetContractStateSlotHash {
    pub condition: SymFeltRef,
    pub slot_index: SymFeltRef,
    pub value: [SymFeltRef; 4],
}

impl DPNStateCmdCore for DPNStateCmdSetContractStateSlotHash {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdSetContractStateSlotSingle {
    pub condition: SymFeltRef,
    pub sub_slot_index: SymFeltRef,
    pub value: SymFeltRef,
}

impl DPNStateCmdCore for DPNStateCmdSetContractStateSlotSingle {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdSetContractStateSlotRange {
    pub condition: SymFeltRef,
    pub sub_slot_index: SymFeltRef,
    pub value: Vec<SymFeltRef>,
}

impl DPNStateCmdCore for DPNStateCmdSetContractStateSlotRange {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdInvokeExternalContractFunction {
    pub condition: SymFeltRef,
    pub contract_id: SymFeltRef,
    pub method_id: SymFeltRef,
    pub input_args: Vec<SymFeltRef>,
    pub num_outputs: u32,
}

impl DPNStateCmdCore for DPNStateCmdInvokeExternalContractFunction {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdGetSelfUserCurrentContractStateSlotHash {
    pub slot_index: SymFeltRef,
}
impl DPNStateCmdGetSelfUserCurrentContractStateSlotHash {
    pub fn new(slot_index: SymFeltRef) -> Self {
        Self {
            slot_index
        }
    }
}
impl DPNStateCmdCore for DPNStateCmdGetSelfUserCurrentContractStateSlotHash {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdGetSelfUserCurrentContractStateSlotSingle {
    pub sub_slot_index: SymFeltRef,
}
impl DPNStateCmdGetSelfUserCurrentContractStateSlotSingle {
    pub fn new(sub_slot_index: SymFeltRef) -> Self {
        Self {
            sub_slot_index
        }
    }
}
impl DPNStateCmdCore for DPNStateCmdGetSelfUserCurrentContractStateSlotSingle {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdGetSelfUserCurrentContractStateSlotRange {
    pub sub_slot_index: SymFeltRef,
    pub length: u32,
}
impl DPNStateCmdGetSelfUserCurrentContractStateSlotRange {
    pub fn new(sub_slot_index: SymFeltRef, length: u32) -> Self {
        Self {
            sub_slot_index,
            length,
        }
    }
}
impl DPNStateCmdCore for DPNStateCmdGetSelfUserCurrentContractStateSlotRange {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdGetSelfUserExternalContractStateSlotHash {
    pub contract_id: SymFeltRef,
    pub slot_index: SymFeltRef,
    pub contract_state_tree_height: u8,
}
impl DPNStateCmdGetSelfUserExternalContractStateSlotHash {
    pub fn new(contract_id: SymFeltRef, contract_state_tree_height: u8, slot_index: SymFeltRef) -> Self {
        Self {
            contract_id,
            contract_state_tree_height,
            slot_index
        }
    }
}
impl DPNStateCmdCore for DPNStateCmdGetSelfUserExternalContractStateSlotHash {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdGetSelfUserExternalContractStateSlotSingle {
    pub contract_id: SymFeltRef,
    pub sub_slot_index: SymFeltRef,
    pub contract_state_tree_height: u8,
}
impl DPNStateCmdGetSelfUserExternalContractStateSlotSingle {
    pub fn new(contract_id: SymFeltRef, contract_state_tree_height: u8, sub_slot_index: SymFeltRef) -> Self {
        Self {
            contract_id,
            contract_state_tree_height,
            sub_slot_index
        }
    }
}
impl DPNStateCmdCore for DPNStateCmdGetSelfUserExternalContractStateSlotSingle {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdGetSelfUserExternalContractStateSlotRange {
    pub contract_id: SymFeltRef,
    pub sub_slot_index: SymFeltRef,
    pub length: u32,
    pub contract_state_tree_height: u8,
}
impl DPNStateCmdGetSelfUserExternalContractStateSlotRange {
    pub fn new(contract_id: SymFeltRef, contract_state_tree_height: u8, sub_slot_index: SymFeltRef, length: u32) -> Self {
        Self {
            contract_id,
            contract_state_tree_height,
            sub_slot_index,
            length,
        }
    }
}
impl DPNStateCmdCore for DPNStateCmdGetSelfUserExternalContractStateSlotRange {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdGetOtherUserContractStateSlotHash {
    pub user_id: SymFeltRef,
    pub contract_id: SymFeltRef,
    pub slot_index: SymFeltRef,
    pub contract_state_tree_height: u8,
}
impl DPNStateCmdGetOtherUserContractStateSlotHash {
    pub fn new(user_id: SymFeltRef, contract_id: SymFeltRef, contract_state_tree_height: u8, slot_index: SymFeltRef) -> Self {
        Self {
            user_id,
            contract_id,
            contract_state_tree_height,
            slot_index
        }
    }
}
impl DPNStateCmdCore for DPNStateCmdGetOtherUserContractStateSlotHash {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdGetOtherUserContractStateSlotSingle {
    pub user_id: SymFeltRef,
    pub contract_id: SymFeltRef,
    pub sub_slot_index: SymFeltRef,
    pub contract_state_tree_height: u8,
}
impl DPNStateCmdGetOtherUserContractStateSlotSingle {
    pub fn new(user_id: SymFeltRef, contract_id: SymFeltRef, contract_state_tree_height: u8, sub_slot_index: SymFeltRef) -> Self {
        Self {
            user_id,
            contract_id,
            contract_state_tree_height,
            sub_slot_index
        }
    }
}
impl DPNStateCmdCore for DPNStateCmdGetOtherUserContractStateSlotSingle {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub struct DPNStateCmdGetOtherUserContractStateSlotRange {
    pub user_id: SymFeltRef,
    pub contract_id: SymFeltRef,
    pub sub_slot_index: SymFeltRef,
    pub length: u32,
    pub contract_state_tree_height: u8,
}
impl DPNStateCmdGetOtherUserContractStateSlotRange {
    pub fn new(user_id: SymFeltRef, contract_id: SymFeltRef, contract_state_tree_height: u8, sub_slot_index: SymFeltRef, length: u32) -> Self {
        Self {
            user_id,
            contract_id,
            contract_state_tree_height,
            sub_slot_index,
            length,
        }
    }
}
impl DPNStateCmdCore for DPNStateCmdGetOtherUserContractStateSlotRange {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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
pub enum DPNStateCmd {
    SetContractStateSlotHash(DPNStateCmdSetContractStateSlotHash),
    SetContractStateSlotSingle(DPNStateCmdSetContractStateSlotSingle),
    SetContractStateSlotRange(DPNStateCmdSetContractStateSlotRange),
    InvokeExternalContractFunction(DPNStateCmdInvokeExternalContractFunction),
    GetSelfUserCurrentContractStateSlotHash(DPNStateCmdGetSelfUserCurrentContractStateSlotHash),
    GetSelfUserCurrentContractStateSlotSingle(DPNStateCmdGetSelfUserCurrentContractStateSlotSingle),
    GetSelfUserCurrentContractStateSlotRange(DPNStateCmdGetSelfUserCurrentContractStateSlotRange),
    GetSelfUserExternalContractStateSlotHash(DPNStateCmdGetSelfUserExternalContractStateSlotHash),
    GetSelfUserExternalContractStateSlotSingle(DPNStateCmdGetSelfUserExternalContractStateSlotSingle),
    GetSelfUserExternalContractStateSlotRange(DPNStateCmdGetSelfUserExternalContractStateSlotRange),
    GetOtherUserContractStateSlotHash(DPNStateCmdGetOtherUserContractStateSlotHash),
    GetOtherUserContractStateSlotSingle(DPNStateCmdGetOtherUserContractStateSlotSingle),
    GetOtherUserContractStateSlotRange(DPNStateCmdGetOtherUserContractStateSlotRange),
}
impl DPNStateCmd {
    pub fn set_contract_state_slot_hash(condition: SymFeltRef, slot_index: SymFeltRef, value: [SymFeltRef; 4]) -> Self {
        DPNStateCmd::SetContractStateSlotHash(DPNStateCmdSetContractStateSlotHash {
            condition,
            slot_index,
            value
        })
    }
    pub fn set_contract_state_slot_single(condition: SymFeltRef, sub_slot_index: SymFeltRef, value: SymFeltRef) -> Self {
        DPNStateCmd::SetContractStateSlotSingle(DPNStateCmdSetContractStateSlotSingle {
            condition,
            sub_slot_index,
            value
        })
    }
    pub fn set_contract_state_slot_range(condition: SymFeltRef,sub_slot_index: SymFeltRef, value: Vec<SymFeltRef>) -> Self {
        DPNStateCmd::SetContractStateSlotRange(DPNStateCmdSetContractStateSlotRange {
            condition,
            sub_slot_index,
            value
        })
    }
    pub fn invoke_external_contract_function(condition: SymFeltRef, contract_id: SymFeltRef, method_id: SymFeltRef, input_args: Vec<SymFeltRef>, num_outputs: u32) -> Self {
        DPNStateCmd::InvokeExternalContractFunction(DPNStateCmdInvokeExternalContractFunction {
            condition,
            contract_id,
            method_id,
            input_args,
            num_outputs
        })
    }
    pub fn get_self_user_current_contract_state_slot_hash(slot_index: SymFeltRef) -> Self {
        DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(DPNStateCmdGetSelfUserCurrentContractStateSlotHash::new(slot_index))
    }
    pub fn get_self_user_current_contract_state_slot_single(sub_slot_index: SymFeltRef) -> Self {
        DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(DPNStateCmdGetSelfUserCurrentContractStateSlotSingle::new(sub_slot_index))
    }
    pub fn get_self_user_current_contract_state_slot_range(sub_slot_index: SymFeltRef, length: u32) -> Self {
        DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(DPNStateCmdGetSelfUserCurrentContractStateSlotRange::new(sub_slot_index, length))
    }
    pub fn get_self_user_external_contract_state_slot_hash(contract_id: SymFeltRef, contract_state_tree_height: u8, slot_index: SymFeltRef) -> Self {
        DPNStateCmd::GetSelfUserExternalContractStateSlotHash(DPNStateCmdGetSelfUserExternalContractStateSlotHash::new(contract_id, contract_state_tree_height, slot_index))
    }
    pub fn get_self_user_external_contract_state_slot_single(contract_id: SymFeltRef, contract_state_tree_height: u8, sub_slot_index: SymFeltRef) -> Self {
        DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(DPNStateCmdGetSelfUserExternalContractStateSlotSingle::new(contract_id, contract_state_tree_height, sub_slot_index))
    }
    pub fn get_self_user_external_contract_state_slot_range(contract_id: SymFeltRef, contract_state_tree_height: u8, sub_slot_index: SymFeltRef, length: u32) -> Self {
        DPNStateCmd::GetSelfUserExternalContractStateSlotRange(DPNStateCmdGetSelfUserExternalContractStateSlotRange::new(contract_id, contract_state_tree_height, sub_slot_index, length))
    }
    pub fn get_other_user_contract_state_slot_hash(user_id: SymFeltRef, contract_id: SymFeltRef, contract_state_tree_height: u8, slot_index: SymFeltRef) -> Self {
        DPNStateCmd::GetOtherUserContractStateSlotHash(DPNStateCmdGetOtherUserContractStateSlotHash::new(user_id, contract_id, contract_state_tree_height, slot_index))
    }
    pub fn get_other_user_contract_state_slot_single(user_id: SymFeltRef, contract_id: SymFeltRef, contract_state_tree_height: u8, sub_slot_index: SymFeltRef) -> Self {
        DPNStateCmd::GetOtherUserContractStateSlotSingle(DPNStateCmdGetOtherUserContractStateSlotSingle::new(user_id, contract_id, contract_state_tree_height, sub_slot_index))
    }
    pub fn get_other_user_contract_state_slot_range(user_id: SymFeltRef, contract_id: SymFeltRef, contract_state_tree_height: u8, sub_slot_index: SymFeltRef, length: u32) -> Self {
        DPNStateCmd::GetOtherUserContractStateSlotRange(DPNStateCmdGetOtherUserContractStateSlotRange::new(user_id, contract_id, contract_state_tree_height, sub_slot_index, length))
    }
    
}
impl DPNStateCmdCore for DPNStateCmd {
    fn get_inputs(&self) -> Vec<SymFeltRef> {
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