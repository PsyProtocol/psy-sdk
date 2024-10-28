use plonky2::field::{
    goldilocks_field::GoldilocksField,
    types::{Field, Field64, PrimeField64},
};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::ops::Neg;

#[derive(
    Serialize_repr, Deserialize_repr, Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd,
)]
#[repr(u16)]
pub enum OpType {
    InputTarget = 0,

    Constant = 1,
    ConstantTrue = 2,
    ConstantFalse = 3,

    // binary
    Add = 4,
    Sub = 5,
    Mul = 6,
    Div = 7,
    Mod = 8,
    BoolAnd = 9,
    BoolOr = 10,
    Eq = 11,
    Lte = 12,
    Gte = 13,
    Gt = 14,
    Lt = 15,
    BitAnd = 16,
    BitXor = 17,
    BitOr = 18,
    BitShl = 19,
    BitShr = 20,

    // unary
    BoolNot = 30,
    Neg = 31,
    Inverse = 32,

    Select = 40,

    HashNoPad = 45,
    HashPad = 46,

    SplitBits = 50,
    SumBits = 51,

    TargetAt = 52,
}

impl From<u16> for OpType {
    fn from(value: u16) -> Self {
        match value {
            0 => OpType::InputTarget,

            1 => OpType::Constant,
            2 => OpType::ConstantTrue,
            3 => OpType::ConstantFalse,

            // binary
            4 => OpType::Add,
            5 => OpType::Sub,
            6 => OpType::Mul,
            7 => OpType::Div,
            8 => OpType::Mod,
            9 => OpType::BoolAnd,
            10 => OpType::BoolOr,
            11 => OpType::Eq,
            12 => OpType::Lte,
            13 => OpType::Gte,
            14 => OpType::Gt,
            15 => OpType::Lt,
            16 => OpType::BitAnd,
            17 => OpType::BitXor,
            18 => OpType::BitOr,
            19 => OpType::BitShl,
            20 => OpType::BitShr,

            // unary
            30 => OpType::BoolNot,
            31 => OpType::Neg,
            32 => OpType::Inverse,

            40 => OpType::Select,

            45 => OpType::HashNoPad,
            46 => OpType::HashPad,

            50 => OpType::SplitBits,
            51 => OpType::SumBits,

            52 => OpType::TargetAt,
            _ => panic!("Invalid OpType value: {}", value),
        }
    }
}

impl OpType {
    pub fn eval_binary_constant(&self, a: u64, b: u64) -> u64 {
        assert!(
            a < GoldilocksField::ORDER,
            "value {} is not a valid Felt",
            a
        );
        assert!(
            b < GoldilocksField::ORDER,
            "value {} is not a valid Felt",
            b
        );
        match self {
            OpType::Add => (GoldilocksField::from_canonical_u64(a)
                + GoldilocksField::from_canonical_u64(b))
            .to_canonical_u64(),
            OpType::Sub => (GoldilocksField::from_canonical_u64(a)
                - GoldilocksField::from_canonical_u64(b))
            .to_canonical_u64(),
            OpType::Mul => (GoldilocksField::from_canonical_u64(a)
                * GoldilocksField::from_canonical_u64(b))
            .to_canonical_u64(),
            OpType::Div => (GoldilocksField::from_canonical_u64(a)
                / GoldilocksField::from_canonical_u64(b))
            .to_canonical_u64(),
            OpType::Mod => a % b,
            OpType::BoolAnd => (a & b) & 1,
            OpType::BoolOr => (a | b) & 1,
            OpType::Eq => {
                if a == b {
                    1
                } else {
                    0
                }
            }
            OpType::Lte => {
                if a <= b {
                    1
                } else {
                    0
                }
            }
            OpType::Gte => {
                if a >= b {
                    1
                } else {
                    0
                }
            }
            OpType::Gt => {
                if a > b {
                    1
                } else {
                    0
                }
            }
            OpType::Lt => {
                if a < b {
                    1
                } else {
                    0
                }
            }
            OpType::BitAnd => a & b,
            OpType::BitXor => (a ^ b) & GoldilocksField::ORDER,
            OpType::BitOr => (a | b) & GoldilocksField::ORDER,
            OpType::BitShl => (a << b) & GoldilocksField::ORDER,
            OpType::BitShr => (a >> b) & GoldilocksField::ORDER,
            _ => panic!(
                "OpType::eval_binary_constant not implemented for {:?}",
                self
            ),
        }
    }
    pub fn eval_unary_constant(&self, a: u64) -> u64 {
        assert!(
            a < GoldilocksField::ORDER,
            "value {} is not a valid Felt",
            a
        );
        match self {
            OpType::BoolNot => {
                if a == 0 {
                    1
                } else {
                    0
                }
            }
            OpType::Inverse => GoldilocksField::from_noncanonical_u64(a)
                .inverse()
                .to_canonical_u64(),
            OpType::Neg => GoldilocksField::from_noncanonical_u64(a)
                .neg()
                .to_canonical_u64(),
            _ => panic!("OpType::eval_unary_constant not implemented for {:?}", self),
        }
    }
}
