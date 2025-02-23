
use kvq::traits::KVQSerializable;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};


#[derive(
    Serialize_repr,
    Deserialize_repr,
    PartialEq,
    Debug,
    Clone,
    Copy,
    Eq,
    Hash,
    PartialOrd,
    Ord,
)]
#[repr(u8)]
pub enum QEDHashHelperHashType {
    Unknown = 0,
    CheckpointTreeRoot = 1,
    CheckpointTreeLeafHash = 2,
}
impl QEDHashHelperHashType {
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}
impl From<QEDHashHelperHashType> for u8 {
    fn from(value: QEDHashHelperHashType) -> u8 {
        value as u8
    }
}
impl TryFrom<u8> for QEDHashHelperHashType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(QEDHashHelperHashType::Unknown),
            1 => Ok(QEDHashHelperHashType::CheckpointTreeRoot),
            2 => Ok(QEDHashHelperHashType::CheckpointTreeLeafHash),
            _ => Err(anyhow::format_err!("Invalid QEDHashHelperHashType value: {}", value)),
        }
    }
}

impl ToString for QEDHashHelperHashType {
    fn to_string(&self) -> String {
        match *self {
            QEDHashHelperHashType::Unknown => "Unknown".to_string(),
            QEDHashHelperHashType::CheckpointTreeRoot => "CheckpointTreeRoot".to_string(),
            QEDHashHelperHashType::CheckpointTreeLeafHash => "CheckpointTreeLeafHash".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QEDHashHelperResult {
    pub hash_type: QEDHashHelperHashType,
    pub checkpoint_id: u64,
    pub target_id: u64,
}

impl KVQSerializable for QEDHashHelperResult {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut result = Vec::with_capacity(17);
        result.push(self.hash_type.to_u8());
        result.extend_from_slice(&u64::to_be_bytes(self.checkpoint_id));
        result.extend_from_slice(&u64::to_be_bytes(self.target_id));
        Ok(result)
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != 17 {
            anyhow::bail!(
                "expected 17 bytes for deserializing QEDHashHelperResult, got {} bytes",
                bytes.len()
            );
        }
        let hash_type = QEDHashHelperHashType::try_from(bytes[0])?;
        let checkpoint_id = u64::from_be_bytes(bytes[1..9].try_into()?);
        let target_id = u64::from_be_bytes(bytes[9..17].try_into()?);
        Ok(Self {
            hash_type,
            checkpoint_id,
            target_id,
        })
    }
}
impl QEDHashHelperResult {
    pub fn new(hash_type: QEDHashHelperHashType, checkpoint_id: u64, target_id: u64) -> Self {
        Self {
            hash_type,
            checkpoint_id,
            target_id,
        }
    }
    pub fn new_checkpoint_tree_root_hash(checkpoint_id: u64) -> Self {
        Self {
            hash_type: QEDHashHelperHashType::CheckpointTreeRoot,
            checkpoint_id,
            target_id: checkpoint_id,
        }
    }
    pub fn new_checkpoint_leaf_hash(checkpoint_id: u64) -> Self {
        Self {
            hash_type: QEDHashHelperHashType::CheckpointTreeLeafHash,
            checkpoint_id,
            target_id: checkpoint_id,
        }
    }
}