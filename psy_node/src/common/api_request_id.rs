use std::time::Instant;

use chrono::Utc;
use kvq::traits::KVQSerializable;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum QAPIWriteRequestType {
    Unknown = 0,
    RegisterUser = 1,
    DeployContract = 2,

    SubmitUserEndCap = 32,

    NotifyUserPodSubTreeRoot = 64,
}
impl QAPIWriteRequestType {
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}
impl From<QAPIWriteRequestType> for u8 {
    fn from(value: QAPIWriteRequestType) -> u8 {
        value as u8
    }
}
impl TryFrom<u8> for QAPIWriteRequestType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(QAPIWriteRequestType::Unknown),
            1 => Ok(QAPIWriteRequestType::RegisterUser),
            2 => Ok(QAPIWriteRequestType::DeployContract),
            3 => Ok(QAPIWriteRequestType::SubmitUserEndCap),
            4 => Ok(QAPIWriteRequestType::NotifyUserPodSubTreeRoot),
            _ => Err(anyhow::format_err!("Invalid QJobTopic value: {}", value)),
        }
    }
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum QAPIWriteRequestBlobType {
    Generic = 0,
    Input = 1,
    ProofWitness = 2,
    InputProof = 3,
    OutputProof = 4,
    ResultStatus = 5,
    Result = 6,
    ErrorMessage = 7,
}
impl QAPIWriteRequestBlobType {
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}
impl From<QAPIWriteRequestBlobType> for u8 {
    fn from(value: QAPIWriteRequestBlobType) -> u8 {
        value as u8
    }
}
impl TryFrom<u8> for QAPIWriteRequestBlobType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(QAPIWriteRequestBlobType::Generic),
            1 => Ok(QAPIWriteRequestBlobType::Input),
            2 => Ok(QAPIWriteRequestBlobType::ProofWitness),
            3 => Ok(QAPIWriteRequestBlobType::InputProof),
            4 => Ok(QAPIWriteRequestBlobType::OutputProof),
            5 => Ok(QAPIWriteRequestBlobType::ResultStatus),
            6 => Ok(QAPIWriteRequestBlobType::Result),
            7 => Ok(QAPIWriteRequestBlobType::ErrorMessage),
            _ => Err(anyhow::format_err!("Invalid QJobTopic value: {}", value)),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct PsyAPIRequestIdGenerator {
    pub realm_id: u32,
    pub node_id: u32,
}

impl PsyAPIRequestIdGenerator {
    pub fn new(realm_id: u32, node_id: u32) -> Self {
        Self { realm_id, node_id }
    }
    pub fn new_request_id(&self, request_type: QAPIWriteRequestType, data_type: QAPIWriteRequestBlobType) -> PsyAPIWriteRequestId {
        PsyAPIWriteRequestId::new(request_type, data_type, self.realm_id, self.node_id)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct PsyAPIWriteRequestId {
    pub request_type: QAPIWriteRequestType,
    pub data_type: QAPIWriteRequestBlobType,
    pub realm_id: u32,
    pub node_id: u32,
    pub time: u64,
    pub random: u64,
}

impl KVQSerializable for PsyAPIWriteRequestId {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

impl PsyAPIWriteRequestId {
    pub fn new(request_type: QAPIWriteRequestType, data_type: QAPIWriteRequestBlobType, realm_id: u32, node_id: u32) -> Self {
        let random = thread_rng().gen::<u64>();
        let time = Utc::now().timestamp_millis() as u64;

        Self {
            request_type,
            data_type,
            realm_id,
            node_id,
            time,
            random,
        }
    }
}
