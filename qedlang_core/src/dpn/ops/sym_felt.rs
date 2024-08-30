use std::hash::Hasher;

use plonky2::field::{goldilocks_field::GoldilocksField, types::Field64};
use serde::{Deserialize, Serialize};
use twox_hash::xxh3::HasherExt;
use super::op_types::DPNOpType;

pub const SYM_FELT_REF_STORE_TYPE_MASK: u128 = 0xffff0000000000000000000000000000u128;
pub const SYM_FELT_REF_STORE_VALUE_MASK: u128 = 0x0000ffffffffffffffffffffffffffffu128;


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct SymFeltRef(pub u128);

impl SymFeltRef {
    pub fn new_input(index: u64) -> SymFeltRef {
        SymFeltRef((DPNOpType::InputTarget as u128)<<112 | index as u128)
    }
    pub fn new_constant(value: u64) -> SymFeltRef {
        assert!(value < GoldilocksField::ORDER, "Constant value {} is too large", value);
        SymFeltRef((DPNOpType::Constant as u128)<<112 | (value%GoldilocksField::ORDER) as u128)
    }
    pub fn get_constant_value(&self) -> u64 {
        (self.0 & 0xffffffffffffffffu128) as u64
    }
    pub fn get_input_index(&self) -> u64 {
        (self.0 & 0xffffffffffffffffu128) as u64
    }
    pub fn get_target_hash_value(&self) -> u128 {
        self.0 & SYM_FELT_REF_STORE_VALUE_MASK
    }
    pub fn new_valueless(op_type: DPNOpType) -> SymFeltRef {
        SymFeltRef((op_type as u128)<<112)
    }

    pub fn get_op_type(&self) -> DPNOpType {
        ((self.0>>112) as u16).into()
    }
    pub fn needs_store(&self) -> bool {
        ((self.0 >> 112) as u16) > 1 
    }

}

impl From<u8> for SymFeltRef {
    fn from(val: u8) -> SymFeltRef {
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128)<<112))
    }
}
impl From<u16> for SymFeltRef {
    fn from(val: u16) -> SymFeltRef {
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128)<<112))
    }
}
impl From<u32> for SymFeltRef {
    fn from(val: u32) -> SymFeltRef {
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128)<<112))
    }
}

impl From<u64> for SymFeltRef {
    fn from(val: u64) -> SymFeltRef {
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128)<<112))
    }
}

impl From<i32> for SymFeltRef {
    fn from(val: i32) -> SymFeltRef {
        assert!(val >= 0, "Negative values are not supported");
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128)<<112))
    }
}
impl From<i64> for SymFeltRef {
    fn from(val: i64) -> SymFeltRef {
        assert!(val >= 0, "Negative values are not supported");
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128)<<112))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq)]
pub struct SymFeltRefValue {
    pub op_type: DPNOpType,
    pub const_param: u64,
    pub inputs: Vec<SymFeltRef>,
}

impl SymFeltRefValue {
    pub fn get_ref_key(&self) -> SymFeltRef {
        if self.op_type == DPNOpType::Constant || self.op_type == DPNOpType::InputTarget {
            return SymFeltRef(((self.op_type as u128)<<112) | self.const_param as u128);
        }else{
            let mut hasher = twox_hash::Xxh3Hash128::default();
            hasher.write(&bincode::serialize(&self).unwrap());
            SymFeltRef((hasher.finish_ext() & SYM_FELT_REF_STORE_VALUE_MASK) | ((self.op_type as u128)<<112))
        }
    }

}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq)]
pub struct SymFeltDef {
    pub op_type: DPNOpType,
    pub const_param: u64,
    pub inputs: Vec<SymFeltDef>,
}



#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct SymRefAssertion {
    pub left: SymFeltRef,
    pub right: SymFeltRef,
    pub message: &'static str,
}
