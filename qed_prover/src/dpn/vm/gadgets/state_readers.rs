use std::collections::HashMap;

use plonky2::{
    field::extension::Extendable,
    hash::hash_types::{HashOut, HashOutTarget, RichField},
    iop::{target::Target, witness::Witness},
    plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher},
};
use qed_common_circuit::{
    builder::{
        connect::CircuitBuilderConnectHelpers, hash::core::CircuitBuilderHashCore,
        math::core::CircuitBuilderCoreMathHelpers, select::CircuitBuilderSelectHelpers,
    },
    hash::merkle::gadgets::{
        delta_merkle_proof::DeltaMerkleProofGadget, merkle_proof::MerkleProofGadget,
        sub_slot_delta_merkle_proof_batch::SubSlotDeltaMerkleProofBatchGadget,
        sub_slot_merkle_proof_batch::SubSlotMerkleProofBatchGadget,
        historical_root_merkle_proof::HistoricalRootMerkleProofGadget,
    },
    traits::{CreatableTarget, ToTargets},
};
use qed_core::{config::network_constants::{CHECKPOINT_TREE_HEIGHT, DEFERRED_TRANSACTION_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT}, data::{base_types::hash256::Hash256, qhashout::QHashOut}};
use qed_crypto::hash::core::sha256;
use qed_rollup_circuit::gadgets::qdata::{
    checkpoint_state_roots::QEDCheckpointGlobalStateRootsGadget, contract_function_call::DPNProvingSessionSimpleMethodCallGadget, user::QEDUserLeafGadget,
    checkpoint_stats::QEDCheckpointLeafStatsGadget
};
use qedlang_core::dpn::ops::state_cmd::data::DPNStateCmd;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::dpn::vm::ops::SimpleDPNBuilder;

#[derive(Clone, Debug)]
pub struct ClearEntireTreeGadget {
    pub state_tree_height: Target,
    pub zero_hash: HashOutTarget,
}

impl ClearEntireTreeGadget {
    pub fn create_virtual<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        Self {
            state_tree_height: builder.add_virtual_target(),
            zero_hash: builder.add_virtual_hash(),
        }
    }

    pub fn set_witness<W: Witness<F>, F: RichField>(
        &self,
        witness: &mut W,
        state_tree_height: u64,
        zero_hash: QHashOut<F>,
    ) -> anyhow::Result<()> {
        witness.set_target(self.state_tree_height, F::from_canonical_u64(state_tree_height));
        witness.set_hash_target(self.zero_hash, zero_hash.into());
        Ok(())
    }
}

#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
#[repr(u8)]
pub enum StateReaderReferenceKeyType {
    MerkleProof = 0,
    DeltaMerkleProof = 1,
    UserLeaf = 2,
    CheckpointStats = 3,
    CheckpointStateRoots = 4,
    HistoricalProof = 5,
    ClearEntireTree = 6,
}
impl StateReaderReferenceKeyType {
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}
impl From<StateReaderReferenceKeyType> for u8 {
    fn from(value: StateReaderReferenceKeyType) -> u8 {
        value as u8
    }
}
impl TryFrom<u8> for StateReaderReferenceKeyType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(StateReaderReferenceKeyType::MerkleProof),
            1 => Ok(StateReaderReferenceKeyType::DeltaMerkleProof),
            2 => Ok(StateReaderReferenceKeyType::UserLeaf),
            3 => Ok(StateReaderReferenceKeyType::CheckpointStats),
            4 => Ok(StateReaderReferenceKeyType::CheckpointStateRoots),
            5 => Ok(StateReaderReferenceKeyType::HistoricalProof),
            6 => Ok(StateReaderReferenceKeyType::ClearEntireTree),
            _ => Err(anyhow::format_err!(
                "Invalid StateReaderReferenceKeyType value: {}",
                value
            )),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct StateReaderReferenceKey {
    pub gadget_type: StateReaderReferenceKeyType,
    pub gadget_index: usize,
}
impl StateReaderReferenceKey {
    pub fn new_merkle_proof_key(index: usize) -> Self {
        Self {
            gadget_type: StateReaderReferenceKeyType::MerkleProof,
            gadget_index: index,
        }
    }
    pub fn new_delta_merkle_proof_key(index: usize) -> Self {
        Self {
            gadget_type: StateReaderReferenceKeyType::DeltaMerkleProof,
            gadget_index: index,
        }
    }
    pub fn new_user_leaf_key(index: usize) -> Self {
        Self {
            gadget_type: StateReaderReferenceKeyType::UserLeaf,
            gadget_index: index,
        }
    }
    pub fn new_checkpoint_stats_key(index: usize) -> Self {
        Self {
            gadget_type: StateReaderReferenceKeyType::CheckpointStats,
            gadget_index: index,
        }
    }
    pub fn new_clear_entire_tree_key(index: usize) -> Self {
        Self {
            gadget_type: StateReaderReferenceKeyType::ClearEntireTree,
            gadget_index: index,
        }
    }
    pub fn to_u64(&self) -> u64 {
        ((self.gadget_type.to_u8() as u64) << 56u64) | (self.gadget_index as u64)
    }
}
impl From<StateReaderReferenceKey> for u64 {
    fn from(value: StateReaderReferenceKey) -> u64 {
        value.to_u64()
    }
}
impl TryFrom<u64> for StateReaderReferenceKey {
    type Error = anyhow::Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let gadget_type = StateReaderReferenceKeyType::try_from((value >> 56u64) as u8)?;
        let gadget_index = (value & 0x00ffffffffffffffu64) as usize;

        Ok(Self {
            gadget_type,
            gadget_index,
        })
    }
}



#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKInvokeDeferredMethodCall {
    pub condition_target_id: u64,
    pub contract_target_id: u64,
    pub method_target_id: u64,
    pub deferred_tx_counter: u32,
    pub input_target_ids_hash: Hash256,
}
impl CKInvokeDeferredMethodCall {
    pub fn new(condition_target_id: u64, contract_target_id: u64, method_target_id: u64, deferred_tx_counter: u32, input_target_ids: &[u64]) -> Self {
        Self {
            condition_target_id,
            contract_target_id,
            method_target_id,
            deferred_tx_counter,
            input_target_ids_hash: sha256::CoreSha256Hasher::hash_u64s(input_target_ids),
        }
    }

}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadCurrentContractSlot {
    pub slot_target_id: u64,
    pub write_epoch: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadCurrentContractSingle {
    pub sub_slot_target_id: u64,
    pub write_epoch: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadCurrentContractRange {
    pub sub_slot_target_id: u64,
    pub length: u32,
    pub slot_offset_index: u64,
    pub write_epoch: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKWriteCurrentContractSlot {
    pub slot_target_id: u64,
    pub condition_target_id: u64,
    pub write_epoch: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKWriteCurrentContractSingle {
    pub sub_slot_target_id: u64,
    pub condition_target_id: u64,
    pub write_epoch: u32,
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKWriteCurrentContractRange {
    pub sub_slot_target_id: u64,
    pub condition_target_id: u64,
    pub length: u32,
    pub slot_offset_index: u64,
    pub write_epoch: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadSelfUserExternalContractRoot {
    pub contract_target_id: u64,
    pub contract_call_epoch: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadSelfUserExternalContractSlot {
    pub contract_target_id: u64,
    pub slot_target_id: u64,
    pub contract_call_epoch: u32,
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadSelfUserExternalContractSingle {
    pub contract_target_id: u64,
    pub sub_slot_target_id: u64,
    pub contract_call_epoch: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadSelfUserExternalContractRange {
    pub contract_target_id: u64,
    pub sub_slot_target_id: u64,
    pub length: u32,
    pub contract_call_epoch: u32,
    pub slot_offset_index: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadOtherUserLeafHash {
    pub user_target_id: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadOtherUserLeaf {
    pub user_target_id: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadOtherUserContractContractRoot {
    pub user_target_id: u64,
    pub contract_target_id: u64,
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadOtherUserContractContractSlot {
    pub user_target_id: u64,
    pub contract_target_id: u64,
    pub slot_target_id: u64,
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadOtherUserContractContractRange {
    pub user_target_id: u64,
    pub contract_target_id: u64,
    pub sub_slot_target_id: u64,
    pub length: u32,
    pub slot_offset_index: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CKReadOtherUserContractContractSingle {
    pub user_target_id: u64,
    pub contract_target_id: u64,
    pub sub_slot_target_id: u64,
    pub write_epoch: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub enum StateCommandCacheKey {
    InvokeDeferredMethodCall(CKInvokeDeferredMethodCall),
    ReadCurrentContractSlot(CKReadCurrentContractSlot),
    ReadCurrentContractSingle(CKReadCurrentContractSingle),
    ReadCurrentContractRange(CKReadCurrentContractRange),
    WriteCurrentContractSlot(CKWriteCurrentContractSlot),
    WriteCurrentContractSingle(CKWriteCurrentContractSingle),
    WriteCurrentContractRange(CKWriteCurrentContractRange),
    ReadSelfUserExternalContractRoot(CKReadSelfUserExternalContractRoot),
    ReadSelfUserExternalContractSlot(CKReadSelfUserExternalContractSlot),
    ReadSelfUserExternalContractSingle(CKReadSelfUserExternalContractSingle),
    ReadSelfUserExternalContractRange(CKReadSelfUserExternalContractRange),
    ReadOtherUserLeafHash(CKReadOtherUserLeafHash),
    ReadOtherUserLeaf(CKReadOtherUserLeaf),
    ReadOtherUserContractContractRoot(CKReadOtherUserContractContractRoot),
    ReadOtherUserContractContractSlot(CKReadOtherUserContractContractSlot),
    ReadOtherUserContractContractRange(CKReadOtherUserContractContractRange),
    ReadOtherUserContractContractSingle(CKReadOtherUserContractContractSingle),
    GetCheckpointStats(CKGetCheckpointStats),
    ClearEntireTree(CKClearEntireTree),
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CKGetCheckpointStats {
    pub checkpoint_id: u64,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CKClearEntireTree {
    pub condition: u64,
    pub write_epoch: u32,
}
impl StateCommandCacheKey {
    pub fn new_read_current_contract_slot(slot_target_id: u64, write_epoch: u32) -> Self {
        Self::ReadCurrentContractSlot(CKReadCurrentContractSlot {
            slot_target_id,
            write_epoch,
        })
    }
    pub fn new_read_current_contract_single(sub_slot_target_id: u64, write_epoch: u32) -> Self {
        Self::ReadCurrentContractSingle(CKReadCurrentContractSingle {
            sub_slot_target_id,
            write_epoch,
        })
    }
    pub fn new_read_current_contract_range(
        sub_slot_target_id: u64,
        length: u32,
        slot_offset_index: u64,
        write_epoch: u32
    ) -> Self {
        Self::ReadCurrentContractRange(CKReadCurrentContractRange {
            sub_slot_target_id,
            length,
            slot_offset_index,
            write_epoch,
        })
    }
    pub fn new_write_current_contract_slot(
        slot_target_id: u64,
        condition_target_id: u64,
        write_epoch: u32,
    ) -> Self {
        Self::WriteCurrentContractSlot(CKWriteCurrentContractSlot {
            slot_target_id,
            condition_target_id,
            write_epoch,
        })
    }
    pub fn new_write_current_contract_single(
        sub_slot_target_id: u64,
        condition_target_id: u64,
        write_epoch: u32,
    ) -> Self {
        Self::WriteCurrentContractSingle(CKWriteCurrentContractSingle {
            sub_slot_target_id,
            condition_target_id,
            write_epoch,
        })
    }
    pub fn new_write_current_contract_range(
        sub_slot_target_id: u64,
        condition_target_id: u64,
        length: u32,
        slot_offset_index: u64,
        write_epoch: u32,
    ) -> Self {
        Self::WriteCurrentContractRange(CKWriteCurrentContractRange {
            sub_slot_target_id,
            condition_target_id,
            length,
            slot_offset_index,
            write_epoch,
        })
    }
    pub fn new_read_self_user_external_contract_root(
        contract_target_id: u64,
        contract_call_epoch: u32,
    ) -> Self {
        Self::ReadSelfUserExternalContractRoot(CKReadSelfUserExternalContractRoot {
            contract_target_id,
            contract_call_epoch,
        })
    }
    pub fn new_read_self_user_external_contract_slot(
        contract_target_id: u64,
        slot_target_id: u64,
        contract_call_epoch: u32,
    ) -> Self {
        Self::ReadSelfUserExternalContractSlot(CKReadSelfUserExternalContractSlot {
            contract_target_id,
            slot_target_id,
            contract_call_epoch,
        })
    }
    pub fn new_read_self_user_external_contract_single(
        contract_target_id: u64,
        sub_slot_target_id: u64,
        contract_call_epoch: u32,
    ) -> Self {
        Self::ReadSelfUserExternalContractSingle(CKReadSelfUserExternalContractSingle {
            contract_target_id,
            sub_slot_target_id,
            contract_call_epoch,
        })
    }
    pub fn new_read_self_user_external_contract_range(
        contract_target_id: u64,
        sub_slot_target_id: u64,
        contract_call_epoch: u32,
        length: u32,
        slot_offset_index: u64,
    ) -> Self {
        Self::ReadSelfUserExternalContractRange(CKReadSelfUserExternalContractRange {
            contract_target_id,
            sub_slot_target_id,
            contract_call_epoch,
            length,
            slot_offset_index,
        })
    }
    pub fn new_read_other_user_leaf_hash(user_target_id: u64) -> Self {
        Self::ReadOtherUserLeafHash(CKReadOtherUserLeafHash { user_target_id })
    }
    pub fn new_read_other_user_leaf(user_target_id: u64) -> Self {
        Self::ReadOtherUserLeaf(CKReadOtherUserLeaf { user_target_id })
    }
    pub fn new_read_other_user_contract_root(user_target_id: u64, contract_target_id: u64) -> Self {
        Self::ReadOtherUserContractContractRoot(CKReadOtherUserContractContractRoot {
            user_target_id,
            contract_target_id,
        })
    }
    pub fn new_read_other_user_contract_slot(
        user_target_id: u64,
        contract_target_id: u64,
        slot_target_id: u64,
    ) -> Self {
        Self::ReadOtherUserContractContractSlot(CKReadOtherUserContractContractSlot {
            user_target_id,
            contract_target_id,
            slot_target_id,
        })
    }
    pub fn new_read_other_user_contract_range(
        user_target_id: u64,
        contract_target_id: u64,
        sub_slot_target_id: u64,
        length: u32,
        slot_offset_index: u64,
    ) -> Self {
        Self::ReadOtherUserContractContractRange(CKReadOtherUserContractContractRange {
            user_target_id,
            contract_target_id,
            sub_slot_target_id,
            length,
            slot_offset_index,
        })
    }
    pub fn new_read_other_user_contract_single(
        user_id: u64,
        contract_id: u64,
        sub_slot_target_id: u64,
        write_epoch: u32,
    ) -> Self {
        Self::ReadOtherUserContractContractSingle(CKReadOtherUserContractContractSingle {
            user_target_id: user_id,
            contract_target_id: contract_id,
            sub_slot_target_id,
            write_epoch,
        })
    }
    pub fn new_get_checkpoint_stats(checkpoint_id: u64) -> Self {
        Self::GetCheckpointStats(CKGetCheckpointStats {
            checkpoint_id,
        })
    }
    pub fn new_clear_entire_tree_with_condition(condition: u64, write_epoch: u32) -> Self {
        Self::ClearEntireTree(CKClearEntireTree { condition, write_epoch })
    }
}

#[derive(Clone, Debug)]
pub struct StateReaderGadget {
    pub merkle_proofs: Vec<MerkleProofGadget>,
    pub delta_merkle_proofs: Vec<DeltaMerkleProofGadget>,
    pub user_leaves: Vec<QEDUserLeafGadget>,
    pub checkpoint_stats_requests: Vec<QEDCheckpointLeafStatsGadget>,
    pub checkpoint_state_roots_requests: Vec<QEDCheckpointGlobalStateRootsGadget>,
    pub historical_proofs: Vec<HistoricalRootMerkleProofGadget>,
    pub clear_entire_tree_requests: Vec<ClearEntireTreeGadget>,
    pub start_contract_state_root: HashOutTarget,
    pub end_contract_state_root: HashOutTarget,
    pub user_contract_tree_state_root: HashOutTarget,
    pub chain_state_roots: QEDCheckpointGlobalStateRootsGadget,
    pub checkpoint_stats: QEDCheckpointLeafStatsGadget,
    pub checkpoint_tree_root: HashOutTarget,

    pub start_deferred_tx_tree_root: HashOutTarget,
    pub end_deferred_tx_tree_root: HashOutTarget,

    pub session_proof_tree_root: HashOutTarget,

    pub state_cmd_results: Vec<Vec<Target>>,

    pub gadget_map: HashMap<StateCommandCacheKey, StateReaderReferenceKey>,
    pub result_map: HashMap<StateCommandCacheKey, Vec<Target>>,

    pub contract_call_epoch: u32,
    pub deferred_tx_count: u32,
    pub write_epoch: u32,

    pub contract_state_tree_height: usize,
    pub session_proof_tree_height: usize,
    pub force_four_align: bool,
}


impl StateReaderGadget {
    pub fn new(
        chain_state_roots: QEDCheckpointGlobalStateRootsGadget,
        user_contract_tree_state_root: HashOutTarget,
        deferred_tx_tree_root: HashOutTarget,
        contract_state_root: HashOutTarget,
        contract_state_tree_height: usize,
        session_proof_tree_root: HashOutTarget,
        session_proof_tree_height: usize,
        force_four_align: bool,
        checkpoint_stats: QEDCheckpointLeafStatsGadget,
        checkpoint_tree_root: HashOutTarget,
    ) -> Self {
        Self {
            merkle_proofs: vec![],
            delta_merkle_proofs: vec![],
            user_leaves: vec![],
            checkpoint_stats_requests: vec![],
            checkpoint_state_roots_requests: vec![],
            historical_proofs: vec![],
            clear_entire_tree_requests: vec![],
            start_deferred_tx_tree_root: deferred_tx_tree_root,
            end_deferred_tx_tree_root: deferred_tx_tree_root,
            start_contract_state_root: contract_state_root,
            end_contract_state_root: contract_state_root,
            chain_state_roots,
            checkpoint_stats,
            checkpoint_tree_root,
            state_cmd_results: vec![],
            gadget_map: HashMap::new(),
            result_map: HashMap::new(),
            contract_call_epoch: 0,
            write_epoch: 0,
            contract_state_tree_height,
            user_contract_tree_state_root,
            deferred_tx_count: 0,

            session_proof_tree_root,
            session_proof_tree_height,
            force_four_align,
        }
    }

    // resolvers

    pub fn resolve_merkle_proof_gadget(
        &self,
        key: &StateCommandCacheKey,
    ) -> Option<&MerkleProofGadget> {
        if self.gadget_map.contains_key(key) {
            let value = self.gadget_map[key];
            if value.gadget_type == StateReaderReferenceKeyType::MerkleProof {
                return Some(&self.merkle_proofs[value.gadget_index]);
            }
        }
        None
    }
    pub fn insert_merkle_proof_gadget(
        &mut self,
        key: StateCommandCacheKey,
        gadget: MerkleProofGadget,
    ) -> StateReaderReferenceKey {
        let ref_key = StateReaderReferenceKey::new_merkle_proof_key(self.merkle_proofs.len());
        self.merkle_proofs.push(gadget);
        self.gadget_map.insert(key, ref_key);
        ref_key
    }

    pub fn resolve_or_insert_merkle_proof_gadget<
        H:AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        key: StateCommandCacheKey,
        height: usize,
    ) -> (bool, &MerkleProofGadget) {
        if self.gadget_map.contains_key(&key) {
            let value = self.gadget_map[&key];
            if value.gadget_type == StateReaderReferenceKeyType::MerkleProof {
                return (false, &self.merkle_proofs[value.gadget_index]);
            }
        }
        let g = MerkleProofGadget::add_virtual_to::<H, F, D>(builder, height);
        self.insert_merkle_proof_gadget(key, g);
        (true, self.merkle_proofs.last().unwrap())
    }

    pub fn resolve_delta_merkle_proof_gadget(
        &self,
        key: &StateCommandCacheKey,
    ) -> Option<&DeltaMerkleProofGadget> {
        if self.gadget_map.contains_key(key) {
            let value = self.gadget_map[key];
            if value.gadget_type == StateReaderReferenceKeyType::DeltaMerkleProof {
                return Some(&self.delta_merkle_proofs[value.gadget_index]);
            }
        }
        None
    }
    pub fn insert_delta_merkle_proof_gadget(
        &mut self,
        key: StateCommandCacheKey,
        gadget: DeltaMerkleProofGadget,
    ) -> StateReaderReferenceKey {
        let ref_key =
            StateReaderReferenceKey::new_delta_merkle_proof_key(self.delta_merkle_proofs.len());
        self.delta_merkle_proofs.push(gadget);
        self.gadget_map.insert(key, ref_key);
        ref_key
    }
    pub fn resolve_or_insert_delta_merkle_proof_gadget<
        H:AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        key: StateCommandCacheKey,
        height: usize,
    ) -> &DeltaMerkleProofGadget {
        if self.gadget_map.contains_key(&key) {
            let value = self.gadget_map[&key];
            if value.gadget_type == StateReaderReferenceKeyType::DeltaMerkleProof {
                return &self.delta_merkle_proofs[value.gadget_index];
            }
        }
        let g = DeltaMerkleProofGadget::add_virtual_to::<H, F, D>(builder, height);
        self.insert_delta_merkle_proof_gadget(key, g);
        self.delta_merkle_proofs.last().unwrap()
    }

    pub fn resolve_user_leaf_gadget(
        &self,
        key: &StateCommandCacheKey,
    ) -> Option<&QEDUserLeafGadget> {
        if self.gadget_map.contains_key(key) {
            let value = self.gadget_map[key];
            if value.gadget_type == StateReaderReferenceKeyType::UserLeaf {
                return Some(&self.user_leaves[value.gadget_index]);
            }
        }
        None
    }
    pub fn insert_user_leaf_gadget(
        &mut self,
        key: StateCommandCacheKey,
        gadget: QEDUserLeafGadget,
    ) -> StateReaderReferenceKey {
        let ref_key = StateReaderReferenceKey::new_user_leaf_key(self.user_leaves.len());
        self.user_leaves.push(gadget);
        self.gadget_map.insert(key, ref_key);
        ref_key
    }

    pub fn insert_checkpoint_stats_gadget(
        &mut self,
        key: StateCommandCacheKey,
        stats_gadget: QEDCheckpointLeafStatsGadget,
        state_roots_gadget: QEDCheckpointGlobalStateRootsGadget,
        historical_proof: HistoricalRootMerkleProofGadget,
    ) -> StateReaderReferenceKey {
        let index = self.checkpoint_stats_requests.len();
        self.checkpoint_stats_requests.push(stats_gadget);
        self.checkpoint_state_roots_requests.push(state_roots_gadget);
        self.historical_proofs.push(historical_proof);
        let ref_key = StateReaderReferenceKey::new_checkpoint_stats_key(index);
        self.gadget_map.insert(key, ref_key);
        ref_key
    }
    pub fn resolve_or_insert_user_leaf_gadget<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        key: StateCommandCacheKey,
    ) -> (bool, &QEDUserLeafGadget) {
        if self.gadget_map.contains_key(&key) {
            let value = self.gadget_map[&key];
            if value.gadget_type == StateReaderReferenceKeyType::UserLeaf {
                return (false, &self.user_leaves[value.gadget_index]);
            }
        }

        let g = QEDUserLeafGadget::create_virtual(builder);
        self.insert_user_leaf_gadget(key, g);
        (true, self.user_leaves.last().unwrap())
    }

    // end resolvers

    pub fn get_self_user_external_contract_root<
        H:AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        dpn: &SimpleDPNBuilder<F, D>,
        contract_target_id: u64,
    ) -> HashOutTarget {
        let read_root_ck = StateCommandCacheKey::new_read_self_user_external_contract_root(
            contract_target_id,
            self.write_epoch,
        );
        let uct_root = self.user_contract_tree_state_root;
        let expected_contract_state_tree_root = {
            let (is_new_uct, mp_uct) = self.resolve_or_insert_merkle_proof_gadget::<H, F, D>(
                builder,
                read_root_ck,
                GLOBAL_CONTRACT_TREE_HEIGHT as usize,
            );
            let expected_contract_state_tree_root = mp_uct.value.clone();
            if is_new_uct {
                builder.connect_hashes(mp_uct.root, uct_root);

                let contract_id_target = dpn.resolve_target(contract_target_id);

                builder.connect(mp_uct.index, contract_id_target);
                let mp_uct: Vec<Target> = mp_uct.value.elements.to_vec();
                self.result_map.insert(read_root_ck, mp_uct);
            }
            expected_contract_state_tree_root
        };
        expected_contract_state_tree_root
    }

    pub fn get_self_user_external_contract_slot_hash<
        H:AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        dpn: &SimpleDPNBuilder<F, D>,
        contract_target_id: u64,
        slot_target_id: u64,
        contract_state_tree_height: usize,
    ) -> HashOutTarget {
        let contract_state_tree_ck =
            StateCommandCacheKey::new_read_self_user_external_contract_slot(
                contract_target_id,
                slot_target_id,
                self.contract_call_epoch,
            );
        let (is_new_contract_state_tree, mp_cst_value, mp_cst_root, mp_cst_index) = {
            let (is_new_contract_state_tree, mp_cst) = self
                .resolve_or_insert_merkle_proof_gadget::<H, F, D>(
                    builder,
                    contract_state_tree_ck,
                    contract_state_tree_height as usize,
                );
            (
                is_new_contract_state_tree,
                mp_cst.value,
                mp_cst.root,
                mp_cst.index,
            )
        };

        let slot_value = mp_cst_value;
        if is_new_contract_state_tree {
            let expected_contract_state_tree_root = self
                .get_self_user_external_contract_root::<H, F, D>(builder, dpn, contract_target_id);
            builder.connect_hashes(mp_cst_root, expected_contract_state_tree_root);

            let slot_index = dpn.resolve_target(slot_target_id);
            builder.connect(slot_index, mp_cst_index);
            self.result_map
                .insert(contract_state_tree_ck, slot_value.elements.to_vec());
        }
        slot_value
    }


    pub fn get_other_user_leaf_hash<
        H:AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        dpn: &SimpleDPNBuilder<F, D>,
        user_target_id: u64,
    ) -> HashOutTarget {
        let user_tree_ck =
            StateCommandCacheKey::new_read_other_user_leaf_hash(
                user_target_id
            );
        let (is_new, mp_user_tree_root, mp_user_tree_value, mp_user_tree_index) = {
            let (is_new, mp_user_tree) = self
                .resolve_or_insert_merkle_proof_gadget::<H, F, D>(
                    builder,
                    user_tree_ck,
                    GLOBAL_USER_TREE_HEIGHT as usize,
                );
            (
                is_new,
                mp_user_tree.root,
                mp_user_tree.value,
                mp_user_tree.index,
            )
        };


        if is_new {
            builder.connect_hashes(mp_user_tree_root, self.chain_state_roots.user_tree_root);
            let user_id_target = dpn.resolve_target(user_target_id);
            builder.connect(mp_user_tree_index, user_id_target);
        }
        mp_user_tree_value
    }


    pub fn get_other_user_leaf<
        H:AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        dpn: &SimpleDPNBuilder<F, D>,
        user_target_id: u64,
    ) -> &QEDUserLeafGadget {
        let expected_leaf_hash = {
            self.get_other_user_leaf_hash::<H, F, D>(builder, dpn, user_target_id)
        };
        let user_leaf_ck =
            StateCommandCacheKey::new_read_other_user_leaf(
                user_target_id
            );

            let (is_new, leaf) = self.resolve_or_insert_user_leaf_gadget::<F, D>(builder, user_leaf_ck);


        if is_new {
            let actual_leaf_hash = leaf.to_hash::<H, F, D>(builder);

            builder.connect_hashes(expected_leaf_hash, actual_leaf_hash);
        }
        leaf
    }




    pub fn get_other_user_contract_state_root<
        H:AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        dpn: &SimpleDPNBuilder<F, D>,
        user_target_id: u64,
        contract_target_id: u64,
    ) -> HashOutTarget {
        let expected_user_contract_tree_root = {
            self.get_other_user_leaf::<H, F, D>(builder, dpn, user_target_id).user_state_tree_root
        };
        let uct_ck =
            StateCommandCacheKey::new_read_other_user_contract_root(
                user_target_id,
                contract_target_id,
            );

        let (is_new, mp_uct) = self.resolve_or_insert_merkle_proof_gadget::<H, F, D>(builder, uct_ck, GLOBAL_CONTRACT_TREE_HEIGHT as usize);


        if is_new {
            let expected_contract_id = dpn.resolve_target(contract_target_id);

            builder.connect(expected_contract_id, mp_uct.index);

            builder.connect_hashes(expected_user_contract_tree_root, mp_uct.root);
        }
        mp_uct.value
    }
    pub fn get_other_user_contract_state_slot_hash<
        H:AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        dpn: &SimpleDPNBuilder<F, D>,
        user_target_id: u64,
        contract_target_id: u64,
        slot_target_id: u64,
        contract_state_tree_height: usize,
    ) -> HashOutTarget {
        let expected_contract_state_tree_root = {
            self.get_other_user_contract_state_root::<H, F, D>(builder, dpn, user_target_id, contract_target_id)
        };

        let cst_ck =
            StateCommandCacheKey::new_read_other_user_contract_slot(
                user_target_id,
                contract_target_id,
                slot_target_id,
            );

        let (is_new, mp_cst) = self.resolve_or_insert_merkle_proof_gadget::<H, F, D>(builder, cst_ck, contract_state_tree_height);


        if is_new {
            let expected_contract_id = dpn.resolve_target(slot_target_id);


            builder.connect(expected_contract_id, mp_cst.index);

            builder.connect_hashes(expected_contract_state_tree_root, mp_cst.root);
        }
        mp_cst.value
    }

    pub fn get_other_user_contract_state_slot_range<
        H: AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        dpn: &SimpleDPNBuilder<F, D>,
        user_target_id: u64,
        contract_target_id: u64,
        sub_slot_target_id: u64,
        contract_state_tree_height: usize,
        length: usize,
    ) -> Vec<Target> {
        let expected_contract_state_tree_root = {
            self.get_other_user_contract_state_root::<H, F, D>(
                builder,
                dpn,
                user_target_id,
                contract_target_id,
            )
        };

        let r_ck = StateCommandCacheKey::new_read_other_user_contract_range(
            user_target_id,
            contract_target_id,
            sub_slot_target_id,
            length as u32,
            0,
        );

        if self.result_map.contains_key(&r_ck) {
            self.result_map.get(&r_ck).unwrap().to_owned()
        } else {
            let sub_slot_index = dpn.resolve_target(sub_slot_target_id);
            let (values, mps) = {
                let gadget = SubSlotMerkleProofBatchGadget::add_virtual_to::<H, F, D>(
                    builder,
                    contract_state_tree_height,
                    length as usize,
                    sub_slot_index,
                    self.force_four_align,
                );
                (gadget.values, gadget.merkle_proof_gadgets)
            };
            builder.connect_hashes(mps[0].root, expected_contract_state_tree_root);
            for (i, mp) in mps.into_iter().enumerate() {
                let ck = StateCommandCacheKey::new_read_other_user_contract_range(
                    user_target_id,
                    contract_target_id,
                    sub_slot_target_id,
                    length as u32,
                    i as u64,
                );
                let _ref_key = self.insert_merkle_proof_gadget(ck, mp);
            }
            self.result_map.insert(r_ck, values.clone());
            values
        }
    }

    pub fn get_self_user_external_contract_state_slot_single<
        H: AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        dpn: &SimpleDPNBuilder<F, D>,
        contract_target_id: u64,
        sub_slot_target_id: u64,
        contract_state_tree_height: usize,
    ) -> Target {
        let ck = StateCommandCacheKey::new_read_self_user_external_contract_single(
            contract_target_id,
            sub_slot_target_id,
            self.contract_call_epoch,
        );

        let expected_contract_state_tree_root = self.get_self_user_external_contract_root::<H, F, D>(
            builder,
            dpn,
            contract_target_id,
        );

        let (is_new, mp_cst) = self.resolve_or_insert_merkle_proof_gadget::<H, F, D>(
            builder,
            ck,
            contract_state_tree_height,
        );

        if is_new {
            let sub_slot_index = dpn.resolve_target(sub_slot_target_id);
            let (slot_index, inner_index) = builder.div_rem4(sub_slot_index);
            let single_value = builder.select_in_hash(mp_cst.value, inner_index);
            builder.connect_hashes(mp_cst.root, expected_contract_state_tree_root);
            builder.connect(slot_index, mp_cst.index);
            self.result_map.insert(ck, vec![single_value]);
            single_value
        } else {
            self.result_map[&ck][0]
        }
    }

    pub fn get_self_user_external_contract_state_slot_range<
        H: AlgebraicHasher<F>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        dpn: &SimpleDPNBuilder<F, D>,
        contract_target_id: u64,
        sub_slot_target_id: u64,
        contract_state_tree_height: usize,
        length: usize,
    ) -> Vec<Target> {
        let r_ck = StateCommandCacheKey::new_read_self_user_external_contract_range(
            contract_target_id,
            sub_slot_target_id,
            length as u32,
            self.contract_call_epoch,
            0,
        );

        if self.result_map.contains_key(&r_ck) {
            self.result_map.get(&r_ck).unwrap().to_owned()
        } else {
            let expected_contract_state_tree_root = self.get_self_user_external_contract_root::<H, F, D>(
                builder,
                dpn,
                contract_target_id,
            );

            let sub_slot_index = dpn.resolve_target(sub_slot_target_id);
            let (values, mps) = {
                let gadget = SubSlotMerkleProofBatchGadget::add_virtual_to::<H, F, D>(
                    builder,
                    contract_state_tree_height,
                    length,
                    sub_slot_index,
                    self.force_four_align,
                );
                (gadget.values, gadget.merkle_proof_gadgets)
            };

            builder.connect_hashes(mps[0].root, expected_contract_state_tree_root);
            for (i, mp) in mps.into_iter().enumerate() {
                let ck = StateCommandCacheKey::new_read_self_user_external_contract_range(
                    contract_target_id,
                    sub_slot_target_id,
                    length as u32,
                    self.contract_call_epoch,
                    i as u64,
                );
                let _ref_key = self.insert_merkle_proof_gadget(ck, mp);
            }

            self.result_map.insert(r_ck, values.clone());
            values
        }
    }




    pub fn injest_symbolic_state_command<
        H:AlgebraicHasher<F> + qed_crypto::hash::traits::hasher::MerkleZeroHasher<HashOut<F>>,
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        dpn: &SimpleDPNBuilder<F, D>,
        cmd: &DPNStateCmd<u64>,
    ) -> Vec<Target> {
        let value = match cmd {
            DPNStateCmd::SetContractStateSlotHash(c) => {
                let dmp = DeltaMerkleProofGadget::add_virtual_to::<H, F, D>(
                    builder,
                    self.contract_state_tree_height,
                );

                let value = HashOutTarget {
                    elements: dpn.resolve_targets_sized::<4>(&c.value),
                };
                let condition = dpn.resolve_bool(builder, c.condition);
                let slot_index = dpn.resolve_target(c.slot_index);

                builder.connect_hashes_if_true(
                    condition,
                    dmp.old_root,
                    self.end_contract_state_root,
                );
                builder.connect_hashes_if_true(condition, dmp.new_value, value);
                builder.connect_if_true(condition, dmp.index, slot_index);

                let new_end_state_root =
                    builder.select_hash(condition, dmp.new_root, self.end_contract_state_root);
                self.end_contract_state_root = new_end_state_root;


                let ck = StateCommandCacheKey::new_write_current_contract_slot(
                    c.slot_index,
                    c.condition,
                    self.write_epoch,
                );
                self.write_epoch += 1;

                let _ref_key = self.insert_delta_merkle_proof_gadget(ck, dmp);

                value.elements.to_vec()
            },
            DPNStateCmd::SetContractStateSlotSingle(c) => {
                let dmp = DeltaMerkleProofGadget::add_virtual_to::<H, F, D>(
                    builder,
                    self.contract_state_tree_height,
                );

                let condition = dpn.resolve_bool(builder, c.condition);
                let sub_slot_index = dpn.resolve_target(c.sub_slot_index);
                let value = dpn.resolve_target(c.value);
                let (slot_index, inner_index) = builder.div_rem4(sub_slot_index);
                // modify a single element in the hash (ie. set hash.elements[inner_index] = value)
                let modded_hash = builder.set_target_in_hash(dmp.old_value, inner_index, value);

                builder.connect_hashes_if_true(
                    condition,
                    dmp.old_root,
                    self.end_contract_state_root,
                );
                builder.connect_hashes_if_true(condition, dmp.new_value, modded_hash);
                builder.connect_if_true(condition, dmp.index, slot_index);

                let new_end_state_root =
                    builder.select_hash(condition, dmp.new_root, self.end_contract_state_root);
                self.end_contract_state_root = new_end_state_root;


                let ck = StateCommandCacheKey::new_write_current_contract_single(
                    c.sub_slot_index,
                    c.condition,
                    self.write_epoch,
                );

                self.write_epoch += 1;
                let _ref_key = self.insert_delta_merkle_proof_gadget(ck, dmp);

                vec![value]
            },
            DPNStateCmd::SetContractStateSlotRange(c) => {
                let condition = dpn.resolve_bool(builder, c.condition);
                let sub_slot_index = dpn.resolve_target(c.sub_slot_index);
                let values = dpn.resolve_targets(&c.value);


                let (values, dmps) = {

                let gadget= SubSlotDeltaMerkleProofBatchGadget::add_virtual_to::<H,F,D>(
                    builder,
                    self.contract_state_tree_height,
                    sub_slot_index,
                    values,
                    self.force_four_align
                );
                (gadget.values, gadget.delta_merkle_proof_gadgets)

                };
                builder.connect_hashes_if_true(
                    condition,
                    dmps[0].old_root,
                    self.end_contract_state_root,
                );

                let new_end_state_root =
                    builder.select_hash(condition, dmps.last().unwrap().new_root, self.end_contract_state_root);
                self.end_contract_state_root = new_end_state_root;
                let values_len = values.len() as u32;
                for (i, dmp) in dmps.into_iter().enumerate() {

                    let ck = StateCommandCacheKey::new_write_current_contract_range(
                        c.sub_slot_index,
                        c.condition,
                        values_len,
                        i as u64,
                        self.write_epoch,
                    );
                    let _ref_key = self.insert_delta_merkle_proof_gadget(ck, dmp);
                }


                self.write_epoch += 1;
                values

            },
            DPNStateCmd::InvokeExternalContractFunctionSync(c) => todo!(),
            DPNStateCmd::GetSelfUserCurrentContractStateSlotHash(c) => {
                let ck = StateCommandCacheKey::new_read_current_contract_slot(
                    c.slot_index,
                    self.write_epoch,
                );

                let end_contract_state_root = self.end_contract_state_root;
                let (is_new, mp) = self.resolve_or_insert_merkle_proof_gadget::<H, F, D>(
                    builder,
                    ck,
                    self.contract_state_tree_height,
                );
                let mp_value = mp.value.elements.to_vec();

                if is_new {
                    let slot_index = dpn.resolve_target(c.slot_index);

                    builder.connect(mp.index, slot_index);
                    builder.connect_hashes(mp.root, end_contract_state_root);
                    self.result_map.insert(ck, mp_value.clone());
                    mp_value
                } else {
                    mp_value
                }
            }
            DPNStateCmd::GetSelfUserCurrentContractStateSlotSingle(c) => {
                let ck = StateCommandCacheKey::new_read_current_contract_single(
                    c.sub_slot_index,
                    self.write_epoch,
                );

                let end_contract_state_root = self.end_contract_state_root;
                let (is_new, mp) = self.resolve_or_insert_merkle_proof_gadget::<H, F, D>(
                    builder,
                    ck,
                    self.contract_state_tree_height,
                );

                if is_new {
                    let sub_slot_index = dpn.resolve_target(c.sub_slot_index);
                    let (slot_index, inner_index) = builder.div_rem4(sub_slot_index);
                    let single_value = builder.select_in_hash(mp.value, inner_index);
                    builder.connect(mp.index, slot_index);
                    builder.connect_hashes(mp.root, end_contract_state_root);
                    self.result_map.insert(ck, vec![single_value]);

                    vec![single_value]
                } else {
                    self.result_map[&ck].to_vec()
                }
            }
            DPNStateCmd::GetSelfUserCurrentContractStateSlotRange(c) => {

                let r_ck = StateCommandCacheKey::new_read_current_contract_range(
                    c.sub_slot_index,
                    c.length,
                    0,
                    self.write_epoch,
                );
                if self.result_map.contains_key(&r_ck) {
                    self.result_map.get(&r_ck).unwrap().to_owned()
                }else{
                    let sub_slot_index = dpn.resolve_target(c.sub_slot_index);
                let (values, mps) = {

                    let gadget= SubSlotMerkleProofBatchGadget::add_virtual_to::<H,F,D>(
                        builder,
                        self.contract_state_tree_height,
                        c.length as usize,
                        sub_slot_index,
                        self.force_four_align
                    );
                    (gadget.values, gadget.merkle_proof_gadgets)

                    };
                    builder.connect_hashes(
                        mps[0].root,
                        self.end_contract_state_root,
                    );
                    for (i, mp) in mps.into_iter().enumerate() {

                        let ck = StateCommandCacheKey::new_read_current_contract_range(
                            c.sub_slot_index,
                            c.length,
                            i as u64,
                            self.write_epoch,
                        );
                        let _ref_key = self.insert_merkle_proof_gadget(ck, mp);
                    }
                    self.result_map.insert(r_ck, values.clone());
                    values

                }


            },
            DPNStateCmd::GetSelfUserExternalContractStateSlotHash(c) => {
                let read_root_ck = StateCommandCacheKey::new_read_self_user_external_contract_root(
                    c.contract_id,
                    self.write_epoch,
                );
                let uct_root = self.user_contract_tree_state_root;
                let call_epoch = self.contract_call_epoch;
                let expected_contract_state_tree_root = {
                    let (is_new_uct, mp_uct) = self
                        .resolve_or_insert_merkle_proof_gadget::<H, F, D>(
                            builder,
                            read_root_ck,
                            GLOBAL_CONTRACT_TREE_HEIGHT as usize,
                        );
                    let expected_contract_state_tree_root = mp_uct.value.clone();
                    if is_new_uct {
                        builder.connect_hashes(mp_uct.root, uct_root);

                        let contract_id_target = dpn.resolve_target(c.contract_id);

                        builder.connect(mp_uct.index, contract_id_target);
                        let mp_uct: Vec<Target> = mp_uct.value.elements.to_vec();
                        self.result_map.insert(read_root_ck, mp_uct);
                    }
                    expected_contract_state_tree_root
                };

                let contract_state_tree_ck =
                    StateCommandCacheKey::new_read_self_user_external_contract_slot(
                        c.contract_id,
                        c.slot_index,
                        call_epoch,
                    );

                let (is_new_contract_state_tree, mp_cst) = self
                    .resolve_or_insert_merkle_proof_gadget::<H, F, D>(
                        builder,
                        contract_state_tree_ck,
                        c.contract_state_tree_height as usize,
                    );

                let slot_value = mp_cst.value.elements.to_vec();
                if is_new_contract_state_tree {
                    builder.connect_hashes(mp_cst.root, expected_contract_state_tree_root);
                    let slot_index = dpn.resolve_target(c.slot_index);
                    builder.connect(slot_index, mp_cst.index);
                    self.result_map
                        .insert(contract_state_tree_ck, slot_value.clone());
                }
                slot_value
            }

            DPNStateCmd::GetSelfUserExternalContractStateSlotSingle(c) => {
                let single_value = self.get_self_user_external_contract_state_slot_single::<H, F, D>(
                    builder,
                    dpn,
                    c.contract_id,
                    c.sub_slot_index,
                    c.contract_state_tree_height as usize,
                );
                vec![single_value]
            },
            DPNStateCmd::GetSelfUserExternalContractStateSlotRange(c) => {
                self.get_self_user_external_contract_state_slot_range::<H, F, D>(
                    builder,
                    dpn,
                    c.contract_id,
                    c.sub_slot_index,
                    c.contract_state_tree_height as usize,
                    c.length as usize,
                )
            },
            DPNStateCmd::GetOtherUserContractStateSlotSingle(c) => {
                let ck = StateCommandCacheKey::new_read_other_user_contract_single(
                    c.user_id,
                    c.contract_id,
                    c.sub_slot_index,
                    self.write_epoch,
                );

                let expected_contract_state_tree_root = self.get_other_user_contract_state_root::<H, F, D>(
                    builder,
                    dpn,
                    c.user_id,
                    c.contract_id,
                );

                let (is_new, mp_cst) = self.resolve_or_insert_merkle_proof_gadget::<H, F, D>(
                    builder,
                    ck,
                    c.contract_state_tree_height as usize,
                );

                if is_new {
                    let sub_slot_index = dpn.resolve_target(c.sub_slot_index);
                    let (slot_index, inner_index) = builder.div_rem4(sub_slot_index);
                    let single_value = builder.select_in_hash(mp_cst.value, inner_index);
                    builder.connect_hashes(mp_cst.root, expected_contract_state_tree_root);
                    builder.connect(slot_index, mp_cst.index);
                    self.result_map.insert(ck, vec![single_value]);
                    vec![single_value]
                } else {
                    self.result_map[&ck].to_vec()
                }
            }
            DPNStateCmd::GetOtherUserContractStateSlotRange(c) => self
                .get_other_user_contract_state_slot_range::<H,F,D>(
                    builder,
                    dpn,
                    c.user_id,
                    c.contract_id,
                    c.sub_slot_index,
                    c.contract_state_tree_height as usize,
                    c.length as usize,
                ),
            DPNStateCmd::GetOtherUserContractStateSlotHash(c) => {
                self.get_other_user_contract_state_slot_hash::<H, F, D>(
                    builder,
                    dpn,
                    c.user_id,
                    c.contract_id,
                    c.slot_index,
                    c.contract_state_tree_height as usize
                ).elements.to_vec()
            },
            DPNStateCmd::InvokeExternalContractFunctionDeferred(c) => {
                let ck = StateCommandCacheKey::InvokeDeferredMethodCall(CKInvokeDeferredMethodCall::new(c.condition, c.contract_id, c.method_id, self.deferred_tx_count, &c.input_args));
                let condition_target = dpn.resolve_bool(builder, c.condition);
                let contract_id_target = dpn.resolve_target(c.contract_id);
                let method_id_target = dpn.resolve_target(c.method_id);
                let input_targets = dpn.resolve_targets(&c.input_args);
                let deferred_method_call_gadget = DPNProvingSessionSimpleMethodCallGadget {
                    contract_id: contract_id_target,
                    method_id: method_id_target,
                    inputs: input_targets,
                };

                let tx_hash = deferred_method_call_gadget.to_hash::<H, F, D>(builder);
                let dmp = DeltaMerkleProofGadget::add_virtual_to::<H, F, D>(builder, DEFERRED_TRANSACTION_TREE_HEIGHT as usize);
                builder.connect_hashes_if_true(condition_target, dmp.old_root, self.end_deferred_tx_tree_root);
                let new_root = builder.select_hash(condition_target, dmp.new_root, self.end_deferred_tx_tree_root);
                self.end_deferred_tx_tree_root = new_root;
                builder.connect_hashes_if_true(condition_target, dmp.new_value, tx_hash);
                let zero_hash = builder.constant_hash(HashOut::ZERO);

                builder.connect_hashes_if_true(condition_target, dmp.old_value, zero_hash);

                let _ref_key = self.insert_delta_merkle_proof_gadget(ck, dmp);
                self.deferred_tx_count += 1;

                tx_hash.elements.to_vec()
            },
            DPNStateCmd::GetCheckpointLeafStats(c) => {
                let ck = StateCommandCacheKey::new_get_checkpoint_stats(c.checkpoint_id);

                if let Some(existing_result) = self.result_map.get(&ck) {
                    existing_result.clone()
                } else {
                    let requested_checkpoint_id = dpn.resolve_target(c.checkpoint_id);

                    let requested_checkpoint_stats = QEDCheckpointLeafStatsGadget::create_virtual(builder);

                    let historical_proof = HistoricalRootMerkleProofGadget::add_virtual_to_zero_gt::<H, F, D>(
                        builder,
                        CHECKPOINT_TREE_HEIGHT as usize
                    );

                    builder.connect(historical_proof.index, requested_checkpoint_id);

                    builder.connect_hashes(
                        historical_proof.current_root,
                        self.checkpoint_tree_root
                    );

                    let checkpoint_stats_hash = requested_checkpoint_stats.to_hash::<H, F, D>(builder);

                    let requested_checkpoint_state_roots = QEDCheckpointGlobalStateRootsGadget::create_virtual(builder);
                    let state_roots_hash = requested_checkpoint_state_roots.to_hash::<H, F, D>(builder);

                    let checkpoint_leaf_hash = builder.hash_two_to_one::<H>(state_roots_hash, checkpoint_stats_hash);

                    builder.connect_hashes(
                        historical_proof.current_value,
                        checkpoint_leaf_hash
                    );

                    self.insert_checkpoint_stats_gadget(
                        ck.clone(),
                        requested_checkpoint_stats.clone(),
                        requested_checkpoint_state_roots,
                        historical_proof,
                    );

                    let mut result = Vec::new();

                    result.push(requested_checkpoint_stats.fees_collected);
                    result.push(requested_checkpoint_stats.user_ops_processed);
                    result.push(requested_checkpoint_stats.total_transactions);
                    result.push(requested_checkpoint_stats.slots_modified);
                    result.extend_from_slice(&requested_checkpoint_stats.pm_jobs_completed.to_targets());
                    result.push(requested_checkpoint_stats.block_time);

                    result.extend_from_slice(&requested_checkpoint_stats.random_seed.elements);

                    let pm_rewards = &requested_checkpoint_stats.pm_rewards_commitment;
                    result.extend_from_slice(&pm_rewards.register_users_root.elements);
                    result.extend_from_slice(&pm_rewards.gutas_root.elements);
                    result.extend_from_slice(&pm_rewards.deploy_contracts_root.elements);

                    result.extend_from_slice(&requested_checkpoint_stats.da_challenges_claimed);

                    self.result_map.insert(ck, result.clone());

                    result
                }
            },
            DPNStateCmd::ClearEntireTree(c) => {
                let ck = StateCommandCacheKey::new_clear_entire_tree_with_condition(c.condition, self.write_epoch);

                let result = if let Some(existing_result) = self.result_map.get(&ck) {
                    existing_result.clone()
                } else {
                    let condition = dpn.resolve_bool(builder, c.condition);
                    let zero_hash_constant = builder.constant_hash(H::get_zero_hash(self.contract_state_tree_height));

                    let new_end_state_root = builder.select_hash(condition, zero_hash_constant, self.end_contract_state_root);
                    self.end_contract_state_root = new_end_state_root;

                    let clear_tree_gadget = ClearEntireTreeGadget::create_virtual(builder);
                    let height_constant = builder.constant(F::from_canonical_usize(self.contract_state_tree_height));

                    builder.connect_hashes(clear_tree_gadget.zero_hash, zero_hash_constant);
                    builder.connect(clear_tree_gadget.state_tree_height, height_constant);

                    let index = self.clear_entire_tree_requests.len();
                    self.clear_entire_tree_requests.push(clear_tree_gadget.clone());
                    let ref_key = StateReaderReferenceKey::new_clear_entire_tree_key(index);
                    self.gadget_map.insert(ck.clone(), ref_key);

                    let result_hash = builder.select_hash(condition, zero_hash_constant, self.start_contract_state_root);
                    let result = result_hash.elements.to_vec();
                    self.result_map.insert(ck, result.clone());
                    result
                };

                self.write_epoch += 1;
                result
            },
        };
        value
    }
}
