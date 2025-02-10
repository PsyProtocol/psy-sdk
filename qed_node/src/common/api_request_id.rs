use serde_repr::{Deserialize_repr, Serialize_repr};


#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
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


#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
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


pub struct QEDAPIWriteRequestId {
    pub request_type: QAPIWriteRequestType,
    pub data_type: QAPIWriteRequestBlobType,
    pub node_id: u32,
    pub time: u64,


}