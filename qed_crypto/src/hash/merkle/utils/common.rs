use kvq::traits::KVQSerializable;
use serde::{Deserialize, Serialize};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SimpleMerkleNodeKey {
    pub level: u8,
    pub index: u64,
}

impl SimpleMerkleNodeKey {

    pub fn new_root() -> Self {
        Self {
            level: 0,
            index: 0,
        }
    }
    pub fn new(level: u8, index: u64) -> Self {
        Self {
            level,
            index,
        }
    }
    pub fn sibling(&self) -> Self {
        Self {
            level: self.level,
            index: self.index ^ 1,
        }
    }
    pub fn siblings(&self) -> Vec<Self> {
        let mut result = Vec::with_capacity(self.level as usize);
        let mut current = *self;
        for _ in 0..self.level {
            result.push(current.sibling());
            current = current.parent();
        }
        result
    }
    pub fn parent(&self) -> Self {
        if self.level == 0 {
            return *self;
        }
        Self {
            level: self.level - 1,
            index: self.index >> 1,
        }
    }
}


impl KVQSerializable for SimpleMerkleNodeKey {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let index_bytes = u64::to_be_bytes(self.index);
        Ok(vec![
            self.level,
            index_bytes[0],
            index_bytes[1],
            index_bytes[2],
            index_bytes[3],
            index_bytes[4],
            index_bytes[5],
            index_bytes[6],
            index_bytes[7],
        ])
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() == 9 {
            Ok(Self{
                level: bytes[0],
                index: u64::from_be_bytes(bytes[1..9].try_into().unwrap()),
            })
        }else{
            anyhow::bail!("error deserializing SimpleMerkleNodeKey, expected 9 bytes, got {}", bytes.len());
        }
    }
}