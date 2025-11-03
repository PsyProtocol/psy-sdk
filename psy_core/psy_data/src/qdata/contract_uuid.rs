use std::str::FromStr;

use kvq::traits::{KVQSerializable, ScyllaKey};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContractUUID {
    pub checkpoint_id: u64,
    pub uuid: u64,
}

impl FromStr for ContractUUID {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;
        Self::from_bytes(&bytes)
    }
}

impl ToString for ContractUUID {
    fn to_string(&self) -> String {
        hex::encode(self.to_bytes().unwrap())
    }
}

impl KVQSerializable for ContractUUID {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let checkpoint_id_be_bytes = self.checkpoint_id.to_be_bytes();
        let uuid_be_bytes = self.uuid.to_be_bytes();
        Ok(vec![
            checkpoint_id_be_bytes[0],
            checkpoint_id_be_bytes[1],
            checkpoint_id_be_bytes[2],
            checkpoint_id_be_bytes[3],
            checkpoint_id_be_bytes[4],
            checkpoint_id_be_bytes[5],
            checkpoint_id_be_bytes[6],
            checkpoint_id_be_bytes[7],
            uuid_be_bytes[0],
            uuid_be_bytes[1],
            uuid_be_bytes[2],
            uuid_be_bytes[3],
            uuid_be_bytes[4],
            uuid_be_bytes[5],
            uuid_be_bytes[6],
            uuid_be_bytes[7],
        ])
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != 16 {
            anyhow::bail!("expected 16 bytes for deserializing ContractUUID, got {} bytes", bytes.len());
        }
        let checkpoint_id = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let uuid = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        Ok(ContractUUID { checkpoint_id, uuid })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContractTableIdKey<const TABLE_TYPE: u16> {
    pub checkpoint_id: u64,
    pub uuid: u64,
}

impl<const TABLE_TYPE: u16> KVQSerializable for ContractTableIdKey<TABLE_TYPE> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let checkpoint_id_be_bytes = self.checkpoint_id.to_be_bytes();
        let uuid_be_bytes = self.uuid.to_be_bytes();
        Ok(vec![
            (TABLE_TYPE >> 8) as u8,
            (TABLE_TYPE & 0xff) as u8,
            checkpoint_id_be_bytes[0],
            checkpoint_id_be_bytes[1],
            checkpoint_id_be_bytes[2],
            checkpoint_id_be_bytes[3],
            checkpoint_id_be_bytes[4],
            checkpoint_id_be_bytes[5],
            checkpoint_id_be_bytes[6],
            checkpoint_id_be_bytes[7],
            uuid_be_bytes[0],
            uuid_be_bytes[1],
            uuid_be_bytes[2],
            uuid_be_bytes[3],
            uuid_be_bytes[4],
            uuid_be_bytes[5],
            uuid_be_bytes[6],
            uuid_be_bytes[7],
        ])
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != 18 {
            anyhow::bail!("expected 18 bytes for deserializing ContractTableIdKey, got {} bytes", bytes.len());
        }
        let checkpoint_id = u64::from_be_bytes(bytes[2..10].try_into().unwrap());
        let uuid = u64::from_be_bytes(bytes[10..18].try_into().unwrap());
        Ok(ContractTableIdKey { checkpoint_id, uuid })
    }
}
impl<const TABLE_TYPE: u16> ContractTableIdKey<TABLE_TYPE> {
    pub fn new(contract_uuid: ContractUUID) -> Self {
        ContractTableIdKey {
            checkpoint_id: contract_uuid.checkpoint_id,
            uuid: contract_uuid.uuid,
        }
    }
}

impl<const TABLE_TYPE: u16> ScyllaKey for ContractTableIdKey<TABLE_TYPE> {
    fn get_partition_key(&self) -> Vec<u8> {
        self.checkpoint_id
            .to_be_bytes()
            .iter()
            .chain(self.uuid.to_be_bytes().iter())
            .cloned()
            .collect()
    }

    fn get_clustering_key(&self) -> Option<Vec<u8>> {
        None
    }

    fn get_table_type(&self) -> u16 {
        TABLE_TYPE
    }
}
