use std::hash::Hash;

use plonky2::field::goldilocks_field::GoldilocksField;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::types::{DPNStateCmdCore, DPNStateCommandType};
use crate::dpn::ops::context_trait::{ContextFelt, ToFelts};

// Constants for field sizes
const PM_REWARD_COMMITMENT_SIZE: usize = 12; // 3 roots * 4 field elements each
const DA_CHALLENGE_WINDOW: usize = 14; // Matching psy_config::network_constants::DA_CHALLENGE_WINDOW
const CONTRACT_LEAF_FELT_SIZE: usize = 9;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
pub struct DPNStateCmdSetContractStateSlotSingle<T> {
    pub condition: T,
    pub sub_slot_index: T,
    pub value: T,
}

impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdSetContractStateSlotSingle<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.condition, self.sub_slot_index, self.value]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::SetContractStateSlotSingle
    }

    fn get_output_felt_size(&self) -> usize {
        2
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, TS)]
#[ts(export, concrete(T = GoldilocksField))]
pub struct DPNStateCmdSetContractStateSlotRange<T> {
    pub condition: T,
    pub sub_slot_index: T,
    pub value: Vec<T>,
}

impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdSetContractStateSlotRange<T> {
    fn get_inputs(&self) -> Vec<T> {
        let mut base = Vec::with_capacity(self.value.len() + 2);
        base.push(self.condition);
        base.push(self.sub_slot_index);
        base.extend(self.value.iter());
        base
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::SetContractStateSlotRange
    }

    fn get_output_felt_size(&self) -> usize {
        self.value.len() * 2
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, TS)]
#[ts(export, concrete(T = GoldilocksField))]
pub struct DPNStateCmdClearEntireTree<T> {
    pub condition: T,
}

impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdClearEntireTree<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.condition]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::ClearEntireTree
    }

    fn get_output_felt_size(&self) -> usize {
        4
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, TS)]
#[ts(export, concrete(T = GoldilocksField))]
pub struct DPNStateCmdInvokeExternalContractFunctionSync<T> {
    pub condition: T,
    pub contract_id: T,
    pub method_id: T,
    pub input_args: Vec<T>,
    pub num_outputs: u32,
}

impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdInvokeExternalContractFunctionSync<T> {
    fn get_inputs(&self) -> Vec<T> {
        let mut base = Vec::with_capacity(self.input_args.len() + 3);
        base.push(self.condition);
        base.push(self.contract_id);
        base.push(self.method_id);
        base.extend(self.input_args.iter());
        base
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::InvokeExternalContractFunctionSync
    }

    fn get_output_felt_size(&self) -> usize {
        self.num_outputs as usize
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, TS)]
#[ts(export, concrete(T = GoldilocksField))]
pub struct DPNStateCmdInvokeExternalContractFunctionDeferred<T> {
    pub condition: T,
    pub contract_id: T,
    pub method_id: T,
    pub input_args: Vec<T>,
}

impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdInvokeExternalContractFunctionDeferred<T> {
    fn get_inputs(&self) -> Vec<T> {
        let mut base = Vec::with_capacity(self.input_args.len() + 3);
        base.push(self.condition);
        base.push(self.contract_id);
        base.push(self.method_id);
        base.extend(self.input_args.iter());
        base
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::InvokeExternalContractFunctionDeferred
    }

    fn get_output_felt_size(&self) -> usize {
        4
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
pub struct DPNStateCmdGetSelfUserCurrentContractStateSlotHash<T> {
    pub slot_index: T,
}
impl<T> DPNStateCmdGetSelfUserCurrentContractStateSlotHash<T> {
    pub fn new(slot_index: T) -> Self {
        Self { slot_index }
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
pub struct DPNStateCmdGetSelfUserCurrentContractStateSlotSingle<T> {
    pub sub_slot_index: T,
}
impl<T> DPNStateCmdGetSelfUserCurrentContractStateSlotSingle<T> {
    pub fn new(sub_slot_index: T) -> Self {
        Self { sub_slot_index }
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
pub struct DPNStateCmdGetSelfUserCurrentContractStateSlotRange<T> {
    pub sub_slot_index: T,
    pub length: u32,
}
impl<T> DPNStateCmdGetSelfUserCurrentContractStateSlotRange<T> {
    pub fn new(sub_slot_index: T, length: u32) -> Self {
        Self { sub_slot_index, length }
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
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
            slot_index,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
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
            sub_slot_index,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
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
            slot_index,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
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
            sub_slot_index,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
pub struct DPNStateCmdGetCheckpointLeafStats<T> {
    pub checkpoint_id: T,
}

impl<T> DPNStateCmdGetCheckpointLeafStats<T> {
    pub fn new(checkpoint_id: T) -> Self {
        Self { checkpoint_id }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy, TS)]
#[ts(export, concrete(T = GoldilocksField))]
pub struct DPNStateCmdGetContractLeaf<T> {
    pub contract_id: T,
}

impl<T> DPNStateCmdGetContractLeaf<T> {
    pub fn new(contract_id: T) -> Self {
        Self { contract_id }
    }
}

impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmdGetContractLeaf<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.contract_id]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::GetContractLeaf
    }

    fn get_output_felt_size(&self) -> usize {
        CONTRACT_LEAF_FELT_SIZE
    }
}

impl<T: Ord + Hash + Clone + Copy> DPNStateCmdCore<T> for DPNStateCmdGetCheckpointLeafStats<T> {
    fn get_inputs(&self) -> Vec<T> {
        vec![self.checkpoint_id]
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        DPNStateCommandType::GetCheckpointLeafStats
    }

    fn get_output_felt_size(&self) -> usize {
        // Returns full checkpoint leaf stats: 10 base fields +
        // PM_REWARD_COMMITMENT_SIZE + DA_CHALLENGE_WINDOW
        10 + PM_REWARD_COMMITMENT_SIZE + DA_CHALLENGE_WINDOW
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, TS)]
#[serde(tag = "type")]
#[ts(export, concrete(T = GoldilocksField))]
pub enum DPNStateCmd<T> {
    SetContractStateSlotHash(DPNStateCmdSetContractStateSlotHash<T>),
    SetContractStateSlotSingle(DPNStateCmdSetContractStateSlotSingle<T>),
    SetContractStateSlotRange(DPNStateCmdSetContractStateSlotRange<T>),
    ClearEntireTree(DPNStateCmdClearEntireTree<T>),
    InvokeExternalContractFunctionSync(DPNStateCmdInvokeExternalContractFunctionSync<T>),
    InvokeExternalContractFunctionDeferred(DPNStateCmdInvokeExternalContractFunctionDeferred<T>),
    GetSelfUserCurrentContractStateSlotHash(DPNStateCmdGetSelfUserCurrentContractStateSlotHash<T>),
    GetSelfUserCurrentContractStateSlotSingle(DPNStateCmdGetSelfUserCurrentContractStateSlotSingle<T>),
    GetSelfUserCurrentContractStateSlotRange(DPNStateCmdGetSelfUserCurrentContractStateSlotRange<T>),
    GetSelfUserExternalContractStateSlotHash(DPNStateCmdGetSelfUserExternalContractStateSlotHash<T>),
    GetSelfUserExternalContractStateSlotSingle(DPNStateCmdGetSelfUserExternalContractStateSlotSingle<T>),
    GetSelfUserExternalContractStateSlotRange(DPNStateCmdGetSelfUserExternalContractStateSlotRange<T>),
    GetOtherUserContractStateSlotHash(DPNStateCmdGetOtherUserContractStateSlotHash<T>),
    GetOtherUserContractStateSlotSingle(DPNStateCmdGetOtherUserContractStateSlotSingle<T>),
    GetOtherUserContractStateSlotRange(DPNStateCmdGetOtherUserContractStateSlotRange<T>),
    GetCheckpointLeafStats(DPNStateCmdGetCheckpointLeafStats<T>),
    GetContractLeaf(DPNStateCmdGetContractLeaf<T>),
}
impl<T> DPNStateCmd<T> {
    pub fn set_contract_state_slot_hash(condition: T, slot_index: T, value: [T; 4]) -> Self {
        DPNStateCmd::SetContractStateSlotHash(DPNStateCmdSetContractStateSlotHash {
            condition,
            slot_index,
            value,
        })
    }
    pub fn set_contract_state_slot_single(condition: T, sub_slot_index: T, value: T) -> Self {
        DPNStateCmd::SetContractStateSlotSingle(DPNStateCmdSetContractStateSlotSingle {
            condition,
            sub_slot_index,
            value,
        })
    }
    pub fn set_contract_state_slot_range(condition: T, sub_slot_index: T, value: Vec<T>) -> Self {
        DPNStateCmd::SetContractStateSlotRange(DPNStateCmdSetContractStateSlotRange {
            condition,
            sub_slot_index,
            value,
        })
    }
    pub fn invoke_external_contract_function(condition: T, contract_id: T, method_id: T, input_args: Vec<T>, num_outputs: u32) -> Self {
        DPNStateCmd::InvokeExternalContractFunctionSync(DPNStateCmdInvokeExternalContractFunctionSync {
            condition,
            contract_id,
            method_id,
            input_args,
            num_outputs,
        })
    }
    pub fn invoke_external_contract_function_deferred(condition: T, contract_id: T, method_id: T, input_args: Vec<T>) -> Self {
        DPNStateCmd::InvokeExternalContractFunctionDeferred(DPNStateCmdInvokeExternalContractFunctionDeferred {
            condition,
            contract_id,
            method_id,
            input_args,
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
        DPNStateCmd::GetSelfUserExternalContractStateSlotHash(DPNStateCmdGetSelfUserExternalContractStateSlotHash::<T>::new(
            contract_id,
            contract_state_tree_height,
            slot_index,
        ))
    }
    pub fn get_self_user_external_contract_state_slot_single(contract_id: T, contract_state_tree_height: u8, sub_slot_index: T) -> Self {
        DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(DPNStateCmdGetSelfUserExternalContractStateSlotSingle::<T>::new(
            contract_id,
            contract_state_tree_height,
            sub_slot_index,
        ))
    }
    pub fn get_self_user_external_contract_state_slot_range(contract_id: T, contract_state_tree_height: u8, sub_slot_index: T, length: u32) -> Self {
        DPNStateCmd::GetSelfUserExternalContractStateSlotRange(DPNStateCmdGetSelfUserExternalContractStateSlotRange::<T>::new(
            contract_id,
            contract_state_tree_height,
            sub_slot_index,
            length,
        ))
    }
    pub fn get_other_user_contract_state_slot_hash(user_id: T, contract_id: T, contract_state_tree_height: u8, slot_index: T) -> Self {
        DPNStateCmd::GetOtherUserContractStateSlotHash(DPNStateCmdGetOtherUserContractStateSlotHash::<T>::new(
            user_id,
            contract_id,
            contract_state_tree_height,
            slot_index,
        ))
    }
    pub fn get_other_user_contract_state_slot_single(user_id: T, contract_id: T, contract_state_tree_height: u8, sub_slot_index: T) -> Self {
        DPNStateCmd::GetOtherUserContractStateSlotSingle(DPNStateCmdGetOtherUserContractStateSlotSingle::<T>::new(
            user_id,
            contract_id,
            contract_state_tree_height,
            sub_slot_index,
        ))
    }
    pub fn get_other_user_contract_state_slot_range(
        user_id: T,
        contract_id: T,
        contract_state_tree_height: u8,
        sub_slot_index: T,
        length: u32,
    ) -> Self {
        DPNStateCmd::GetOtherUserContractStateSlotRange(DPNStateCmdGetOtherUserContractStateSlotRange::<T>::new(
            user_id,
            contract_id,
            contract_state_tree_height,
            sub_slot_index,
            length,
        ))
    }
    pub fn get_checkpoint_leaf_stats(checkpoint_id: T) -> Self {
        DPNStateCmd::GetCheckpointLeafStats(DPNStateCmdGetCheckpointLeafStats::<T>::new(checkpoint_id))
    }
    pub fn get_contract_leaf(contract_id: T) -> Self {
        DPNStateCmd::GetContractLeaf(DPNStateCmdGetContractLeaf::<T>::new(contract_id))
    }
}
impl<T: Copy + Clone + Hash + Ord> DPNStateCmdCore<T> for DPNStateCmd<T> {
    fn get_inputs(&self) -> Vec<T> {
        match self {
            DPNStateCmd::SetContractStateSlotHash(c) => c.get_inputs(),
            DPNStateCmd::SetContractStateSlotSingle(c) => c.get_inputs(),
            DPNStateCmd::SetContractStateSlotRange(c) => c.get_inputs(),
            DPNStateCmd::ClearEntireTree(c) => c.get_inputs(),
            DPNStateCmd::InvokeExternalContractFunctionSync(c) => c.get_inputs(),
            DPNStateCmd::InvokeExternalContractFunctionDeferred(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(c) => c.get_inputs(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(c) => c.get_inputs(),
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => c.get_inputs(),
            DPNStateCmd::GetOtherUserContractStateSlotSingle(c) => c.get_inputs(),
            DPNStateCmd::GetOtherUserContractStateSlotRange(c) => c.get_inputs(),
            DPNStateCmd::GetCheckpointLeafStats(c) => c.get_inputs(),
            DPNStateCmd::GetContractLeaf(c) => c.get_inputs(),
        }
    }

    fn get_state_command_type(&self) -> DPNStateCommandType {
        match self {
            DPNStateCmd::SetContractStateSlotHash(c) => c.get_state_command_type(),
            DPNStateCmd::SetContractStateSlotSingle(c) => c.get_state_command_type(),
            DPNStateCmd::SetContractStateSlotRange(c) => c.get_state_command_type(),
            DPNStateCmd::ClearEntireTree(c) => c.get_state_command_type(),
            DPNStateCmd::InvokeExternalContractFunctionSync(c) => c.get_state_command_type(),
            DPNStateCmd::InvokeExternalContractFunctionDeferred(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(c) => c.get_state_command_type(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(c) => c.get_state_command_type(),
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => c.get_state_command_type(),
            DPNStateCmd::GetOtherUserContractStateSlotSingle(c) => c.get_state_command_type(),
            DPNStateCmd::GetOtherUserContractStateSlotRange(c) => c.get_state_command_type(),
            DPNStateCmd::GetCheckpointLeafStats(c) => c.get_state_command_type(),
            DPNStateCmd::GetContractLeaf(c) => c.get_state_command_type(),
        }
    }

    fn get_output_felt_size(&self) -> usize {
        match self {
            DPNStateCmd::SetContractStateSlotHash(c) => c.get_output_felt_size(),
            DPNStateCmd::SetContractStateSlotSingle(c) => c.get_output_felt_size(),
            DPNStateCmd::SetContractStateSlotRange(c) => c.get_output_felt_size(),
            DPNStateCmd::ClearEntireTree(c) => c.get_output_felt_size(),
            DPNStateCmd::InvokeExternalContractFunctionSync(c) => c.get_output_felt_size(),
            DPNStateCmd::InvokeExternalContractFunctionDeferred(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(c) => c.get_output_felt_size(),
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(c) => c.get_output_felt_size(),
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => c.get_output_felt_size(),
            DPNStateCmd::GetOtherUserContractStateSlotSingle(c) => c.get_output_felt_size(),
            DPNStateCmd::GetOtherUserContractStateSlotRange(c) => c.get_output_felt_size(),
            DPNStateCmd::GetCheckpointLeafStats(c) => c.get_output_felt_size(),
            DPNStateCmd::GetContractLeaf(c) => c.get_output_felt_size(),
        }
    }
}

impl<F: ContextFelt> ToFelts<F> for DPNStateCmd<u64> {
    fn to_felts(&self) -> Vec<F> {
        let mut out = Vec::new();
        out.push(F::cns(self.get_state_command_type().get_enc_value() as u64));
        match self {
            DPNStateCmd::SetContractStateSlotHash(cmd) => {
                out.push(F::cns(cmd.condition));
                out.push(F::cns(cmd.slot_index));
                for v in &cmd.value {
                    out.push(F::cns(*v));
                }
            }
            DPNStateCmd::SetContractStateSlotSingle(cmd) => {
                out.push(F::cns(cmd.condition));
                out.push(F::cns(cmd.sub_slot_index));
                out.push(F::cns(cmd.value));
            }
            DPNStateCmd::SetContractStateSlotRange(cmd) => {
                out.push(F::cns(cmd.condition));
                out.push(F::cns(cmd.sub_slot_index));
                out.push(F::cns(cmd.value.len() as u64));
                for v in &cmd.value {
                    out.push(F::cns(*v));
                }
            }
            DPNStateCmd::ClearEntireTree(cmd) => {
                out.push(F::cns(cmd.condition));
            }
            DPNStateCmd::InvokeExternalContractFunctionSync(cmd) => {
                out.push(F::cns(cmd.condition));
                out.push(F::cns(cmd.contract_id));
                out.push(F::cns(cmd.method_id));
                out.push(F::cns(cmd.input_args.len() as u64));
                for v in &cmd.input_args {
                    out.push(F::cns(*v));
                }
                out.push(F::cns(cmd.num_outputs as u64));
            }
            DPNStateCmd::InvokeExternalContractFunctionDeferred(cmd) => {
                out.push(F::cns(cmd.condition));
                out.push(F::cns(cmd.contract_id));
                out.push(F::cns(cmd.method_id));
                out.push(F::cns(cmd.input_args.len() as u64));
                for v in &cmd.input_args {
                    out.push(F::cns(*v));
                }
            }
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(cmd) => {
                out.push(F::cns(cmd.slot_index));
            }
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(cmd) => {
                out.push(F::cns(cmd.sub_slot_index));
            }
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(cmd) => {
                out.push(F::cns(cmd.sub_slot_index));
                out.push(F::cns(cmd.length as u64));
            }
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(cmd) => {
                out.push(F::cns(cmd.contract_id));
                out.push(F::cns(cmd.slot_index));
                out.push(F::cns(cmd.contract_state_tree_height as u64));
            }
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(cmd) => {
                out.push(F::cns(cmd.contract_id));
                out.push(F::cns(cmd.sub_slot_index));
                out.push(F::cns(cmd.contract_state_tree_height as u64));
            }
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(cmd) => {
                out.push(F::cns(cmd.contract_id));
                out.push(F::cns(cmd.sub_slot_index));
                out.push(F::cns(cmd.length as u64));
                out.push(F::cns(cmd.contract_state_tree_height as u64));
            }
            DPNStateCmd::GetOtherUserContractStateSlotHash(cmd) => {
                out.push(F::cns(cmd.user_id));
                out.push(F::cns(cmd.contract_id));
                out.push(F::cns(cmd.slot_index));
                out.push(F::cns(cmd.contract_state_tree_height as u64));
            }
            DPNStateCmd::GetOtherUserContractStateSlotSingle(cmd) => {
                out.push(F::cns(cmd.user_id));
                out.push(F::cns(cmd.contract_id));
                out.push(F::cns(cmd.sub_slot_index));
                out.push(F::cns(cmd.contract_state_tree_height as u64));
            }
            DPNStateCmd::GetOtherUserContractStateSlotRange(cmd) => {
                out.push(F::cns(cmd.user_id));
                out.push(F::cns(cmd.contract_id));
                out.push(F::cns(cmd.sub_slot_index));
                out.push(F::cns(cmd.length as u64));
                out.push(F::cns(cmd.contract_state_tree_height as u64));
            }
            DPNStateCmd::GetCheckpointLeafStats(cmd) => {
                out.push(F::cns(cmd.checkpoint_id));
            }
            DPNStateCmd::GetContractLeaf(cmd) => {
                out.push(F::cns(cmd.contract_id));
            }
        }
        out
    }

    fn from_felts(felts: &[F]) -> Self {
        if felts.is_empty() {
            panic!("DPNStateCmd requires at least one felt");
        }
        let mut idx = 0usize;
        let mut take = |felts: &[F], idx: &mut usize| {
            if *idx >= felts.len() {
                panic!("DPNStateCmd decoding overflow");
            }
            let v = felts[*idx].get_u64();
            *idx += 1;
            v
        };
        let variant = take(felts, &mut idx) as u8;
        match DPNStateCommandType::from(variant) {
            DPNStateCommandType::SetContractStateSlotHash => {
                let condition = take(felts, &mut idx);
                let slot_index = take(felts, &mut idx);
                let mut value = [0u64; 4];
                for item in &mut value {
                    *item = take(felts, &mut idx);
                }
                DPNStateCmd::SetContractStateSlotHash(DPNStateCmdSetContractStateSlotHash {
                    condition,
                    slot_index,
                    value,
                })
            }
            DPNStateCommandType::SetContractStateSlotSingle => {
                let condition = take(felts, &mut idx);
                let sub_slot_index = take(felts, &mut idx);
                let value = take(felts, &mut idx);
                DPNStateCmd::SetContractStateSlotSingle(DPNStateCmdSetContractStateSlotSingle {
                    condition,
                    sub_slot_index,
                    value,
                })
            }
            DPNStateCommandType::SetContractStateSlotRange => {
                let condition = take(felts, &mut idx);
                let sub_slot_index = take(felts, &mut idx);
                let len = take(felts, &mut idx) as usize;
                if felts.len() < idx + len {
                    panic!("SetContractStateSlotRange length mismatch");
                }
                let mut value = Vec::with_capacity(len);
                for _ in 0..len {
                    value.push(take(felts, &mut idx));
                }
                DPNStateCmd::SetContractStateSlotRange(DPNStateCmdSetContractStateSlotRange {
                    condition,
                    sub_slot_index,
                    value,
                })
            }
            DPNStateCommandType::ClearEntireTree => {
                let condition = take(felts, &mut idx);
                DPNStateCmd::ClearEntireTree(DPNStateCmdClearEntireTree { condition })
            }
            DPNStateCommandType::InvokeExternalContractFunctionSync => {
                let condition = take(felts, &mut idx);
                let contract_id = take(felts, &mut idx);
                let method_id = take(felts, &mut idx);
                let len = take(felts, &mut idx) as usize;
                if felts.len() < idx + len {
                    panic!("InvokeExternalContractFunctionSync args length mismatch");
                }
                let mut input_args = Vec::with_capacity(len);
                for _ in 0..len {
                    input_args.push(take(felts, &mut idx));
                }
                let num_outputs = take(felts, &mut idx) as u32;
                DPNStateCmd::InvokeExternalContractFunctionSync(DPNStateCmdInvokeExternalContractFunctionSync {
                    condition,
                    contract_id,
                    method_id,
                    input_args,
                    num_outputs,
                })
            }
            DPNStateCommandType::InvokeExternalContractFunctionDeferred => {
                let condition = take(felts, &mut idx);
                let contract_id = take(felts, &mut idx);
                let method_id = take(felts, &mut idx);
                let len = take(felts, &mut idx) as usize;
                if felts.len() < idx + len {
                    panic!("InvokeExternalContractFunctionDeferred args length mismatch");
                }
                let mut input_args = Vec::with_capacity(len);
                for _ in 0..len {
                    input_args.push(take(felts, &mut idx));
                }
                DPNStateCmd::InvokeExternalContractFunctionDeferred(DPNStateCmdInvokeExternalContractFunctionDeferred {
                    condition,
                    contract_id,
                    method_id,
                    input_args,
                })
            }
            DPNStateCommandType::GetSelfUserCurrentContractStateSlotHash => {
                let slot_index = take(felts, &mut idx);
                DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(DPNStateCmdGetSelfUserCurrentContractStateSlotHash::new(slot_index))
            }
            DPNStateCommandType::GetSelfUserCurrentContractStateSlotSingle => {
                let sub_slot_index = take(felts, &mut idx);
                DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(DPNStateCmdGetSelfUserCurrentContractStateSlotSingle::new(sub_slot_index))
            }
            DPNStateCommandType::GetSelfUserCurrentContractStateSlotRange => {
                let sub_slot_index = take(felts, &mut idx);
                let length = take(felts, &mut idx) as u32;
                DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(DPNStateCmdGetSelfUserCurrentContractStateSlotRange::new(
                    sub_slot_index,
                    length,
                ))
            }
            DPNStateCommandType::GetSelfUserExternalContractStateSlotHash => {
                let contract_id = take(felts, &mut idx);
                let slot_index = take(felts, &mut idx);
                let height = take(felts, &mut idx) as u8;
                DPNStateCmd::GetSelfUserExternalContractStateSlotHash(DPNStateCmdGetSelfUserExternalContractStateSlotHash::new(
                    contract_id,
                    height,
                    slot_index,
                ))
            }
            DPNStateCommandType::GetSelfUserExternalContractStateSlotSingle => {
                let contract_id = take(felts, &mut idx);
                let sub_slot_index = take(felts, &mut idx);
                let height = take(felts, &mut idx) as u8;
                DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(DPNStateCmdGetSelfUserExternalContractStateSlotSingle::new(
                    contract_id,
                    height,
                    sub_slot_index,
                ))
            }
            DPNStateCommandType::GetSelfUserExternalContractStateSlotRange => {
                let contract_id = take(felts, &mut idx);
                let sub_slot_index = take(felts, &mut idx);
                let length = take(felts, &mut idx) as u32;
                let height = take(felts, &mut idx) as u8;
                DPNStateCmd::GetSelfUserExternalContractStateSlotRange(DPNStateCmdGetSelfUserExternalContractStateSlotRange::new(
                    contract_id,
                    height,
                    sub_slot_index,
                    length,
                ))
            }
            DPNStateCommandType::GetOtherUserContractStateSlotHash => {
                let user_id = take(felts, &mut idx);
                let contract_id = take(felts, &mut idx);
                let slot_index = take(felts, &mut idx);
                let height = take(felts, &mut idx) as u8;
                DPNStateCmd::GetOtherUserContractStateSlotHash(DPNStateCmdGetOtherUserContractStateSlotHash::new(
                    user_id,
                    contract_id,
                    height,
                    slot_index,
                ))
            }
            DPNStateCommandType::GetOtherUserContractStateSlotSingle => {
                let user_id = take(felts, &mut idx);
                let contract_id = take(felts, &mut idx);
                let sub_slot_index = take(felts, &mut idx);
                let height = take(felts, &mut idx) as u8;
                DPNStateCmd::GetOtherUserContractStateSlotSingle(DPNStateCmdGetOtherUserContractStateSlotSingle::new(
                    user_id,
                    contract_id,
                    height,
                    sub_slot_index,
                ))
            }
            DPNStateCommandType::GetOtherUserContractStateSlotRange => {
                let user_id = take(felts, &mut idx);
                let contract_id = take(felts, &mut idx);
                let sub_slot_index = take(felts, &mut idx);
                let length = take(felts, &mut idx) as u32;
                let height = take(felts, &mut idx) as u8;
                DPNStateCmd::GetOtherUserContractStateSlotRange(DPNStateCmdGetOtherUserContractStateSlotRange::new(
                    user_id,
                    contract_id,
                    height,
                    sub_slot_index,
                    length,
                ))
            }
            DPNStateCommandType::GetCheckpointLeafStats => {
                let checkpoint_id = take(felts, &mut idx);
                DPNStateCmd::GetCheckpointLeafStats(DPNStateCmdGetCheckpointLeafStats::new(checkpoint_id))
            }
            DPNStateCommandType::GetContractLeaf => {
                let contract_id = take(felts, &mut idx);
                DPNStateCmd::GetContractLeaf(DPNStateCmdGetContractLeaf::new(contract_id))
            }
        }
    }
}

impl<T: Copy + Clone + Hash + Ord> DPNStateCmd<T> {
    pub fn convert_to_u64(&self, inputs_as_u64: &[u64]) -> DPNStateCmd<u64> {
        match self {
            DPNStateCmd::SetContractStateSlotHash(_c) => DPNStateCmd::SetContractStateSlotHash(DPNStateCmdSetContractStateSlotHash {
                condition: inputs_as_u64[0],
                slot_index: inputs_as_u64[1],
                value: [inputs_as_u64[2], inputs_as_u64[3], inputs_as_u64[4], inputs_as_u64[5]],
            }),
            DPNStateCmd::SetContractStateSlotSingle(_c) => DPNStateCmd::SetContractStateSlotSingle(DPNStateCmdSetContractStateSlotSingle {
                condition: inputs_as_u64[0],
                sub_slot_index: inputs_as_u64[1],
                value: inputs_as_u64[2],
            }),
            DPNStateCmd::SetContractStateSlotRange(_c) => DPNStateCmd::SetContractStateSlotRange(DPNStateCmdSetContractStateSlotRange {
                condition: inputs_as_u64[0],
                sub_slot_index: inputs_as_u64[1],
                value: inputs_as_u64[2..].to_vec(),
            }),
            DPNStateCmd::ClearEntireTree(c) => DPNStateCmd::ClearEntireTree(DPNStateCmdClearEntireTree { condition: inputs_as_u64[0] }),
            DPNStateCmd::InvokeExternalContractFunctionSync(c) => {
                DPNStateCmd::InvokeExternalContractFunctionSync(DPNStateCmdInvokeExternalContractFunctionSync {
                    condition: inputs_as_u64[0],
                    contract_id: inputs_as_u64[1],
                    method_id: inputs_as_u64[2],
                    input_args: inputs_as_u64[3..].to_vec(),
                    num_outputs: c.num_outputs,
                })
            }
            DPNStateCmd::InvokeExternalContractFunctionDeferred(_c) => {
                DPNStateCmd::InvokeExternalContractFunctionDeferred(DPNStateCmdInvokeExternalContractFunctionDeferred {
                    condition: inputs_as_u64[0],
                    contract_id: inputs_as_u64[1],
                    method_id: inputs_as_u64[2],
                    input_args: inputs_as_u64[3..].to_vec(),
                })
            }
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(_c) => {
                DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(DPNStateCmdGetSelfUserCurrentContractStateSlotHash::<u64>::new(inputs_as_u64[0]))
            }
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(_c) => DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(
                DPNStateCmdGetSelfUserCurrentContractStateSlotSingle::<u64>::new(inputs_as_u64[0]),
            ),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(c) => DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(
                DPNStateCmdGetSelfUserCurrentContractStateSlotRange::<u64>::new(inputs_as_u64[0], c.length),
            ),
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => DPNStateCmd::GetSelfUserExternalContractStateSlotHash(
                DPNStateCmdGetSelfUserExternalContractStateSlotHash::<u64>::new(inputs_as_u64[0], c.contract_state_tree_height, inputs_as_u64[1]),
            ),
            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(c) => DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(
                DPNStateCmdGetSelfUserExternalContractStateSlotSingle::<u64>::new(inputs_as_u64[0], c.contract_state_tree_height, inputs_as_u64[1]),
            ),
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(c) => {
                DPNStateCmd::GetSelfUserExternalContractStateSlotRange(DPNStateCmdGetSelfUserExternalContractStateSlotRange::<u64>::new(
                    inputs_as_u64[0],
                    c.contract_state_tree_height,
                    inputs_as_u64[1],
                    c.length,
                ))
            }
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => {
                DPNStateCmd::GetOtherUserContractStateSlotHash(DPNStateCmdGetOtherUserContractStateSlotHash::<u64>::new(
                    inputs_as_u64[0],
                    inputs_as_u64[1],
                    c.contract_state_tree_height,
                    inputs_as_u64[2],
                ))
            }
            DPNStateCmd::GetOtherUserContractStateSlotSingle(c) => {
                DPNStateCmd::GetOtherUserContractStateSlotSingle(DPNStateCmdGetOtherUserContractStateSlotSingle::<u64>::new(
                    inputs_as_u64[0],
                    inputs_as_u64[1],
                    c.contract_state_tree_height,
                    inputs_as_u64[2],
                ))
            }
            DPNStateCmd::GetOtherUserContractStateSlotRange(c) => {
                DPNStateCmd::GetOtherUserContractStateSlotRange(DPNStateCmdGetOtherUserContractStateSlotRange::<u64>::new(
                    inputs_as_u64[0],
                    inputs_as_u64[1],
                    c.contract_state_tree_height,
                    inputs_as_u64[2],
                    c.length,
                ))
            }
            DPNStateCmd::GetCheckpointLeafStats(c) => {
                DPNStateCmd::GetCheckpointLeafStats(DPNStateCmdGetCheckpointLeafStats::<u64>::new(inputs_as_u64[0]))
            }
            DPNStateCmd::GetContractLeaf(_c) => DPNStateCmd::GetContractLeaf(DPNStateCmdGetContractLeaf::<u64>::new(inputs_as_u64[0])),
        }
    }
}
