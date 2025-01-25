use qed_common_circuit::hash::merkle::gadgets::{delta_merkle_proof::DeltaMerkleProofGadget, merkle_proof::MerkleProofGadget};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};


#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
#[repr(u8)]
pub enum StateReaderReferenceKeyType {
    MerkleProof = 0,
    DeltaMerkleProof = 1,
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
            _ => Err(anyhow::format_err!("Invalid StateReaderReferenceKeyType value: {}", value)),
        }
    }
}


#[derive(
    Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
pub struct StateReaderReferenceKey {
    pub gadget_type: StateReaderReferenceKeyType,
    pub gadget_index: usize,
}
impl StateReaderReferenceKey {
    pub fn to_u64(&self) -> u64 {
        ((self.gadget_type.to_u8() as u64)<<56u64) | (self.gadget_index as u64)
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
        let gadget_index = (value&0x00ffffffffffffffu64) as usize;



        Ok(Self {
            gadget_type,
            gadget_index,
        })
    }
}

#[derive(
    Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
pub struct CKReadCurrentContractSlot {
    pub slot_target_id: u64,
    pub write_epoch: u32,
}

#[derive(
    Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
pub struct CKWriteCurrentContractSlot {
    pub slot_target_id: u64,
    pub write_epoch: u32,
}


#[derive(
    Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
pub struct CKReadSelfUserExternalContractRoot {
    pub contract_target_id: u64,
    pub contract_call_epoch: u32,
}

#[derive(
    Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
pub struct CKReadSelfUserExternalContractSlot {
    pub contract_target_id: u64,
    pub slot_target_id: u64,
    pub contract_call_epoch: u32,
}


pub struct StateReaderGadget {
    pub merkle_proofs: Vec<MerkleProofGadget>,
    pub delta_merkle_proofs: Vec<DeltaMerkleProofGadget>,
    pub current_write_epoch: u32,
    pub current_contract_call_epoch: u32,
}