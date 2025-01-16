use plonky2::field::{goldilocks_field::GoldilocksField, types::{Field, Field64, PrimeField64}};
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

    Add = 4,
    Sub = 5,
    Mul = 6,
    Div = 7,
    BoolNot = 8,
    BoolAnd = 9,
    BoolOr = 10,
    Xor = 11,
    Nor = 12,
    Eq = 13,
    Lte = 14,
    Gte = 15,
    Gt = 16,
    Lt = 17,
    SplitBits = 18,
    SumBits = 19,
    TargetAt = 20,
    HashNoPad = 21,
    HashPad = 22,
    Select = 23,

    Exp = 24,
    ExpConstantPower = 25,
    ExpConstantBase = 26,

    Mod = 27,
    ModConstantDividend = 28,
    ModConstantDivisor = 29,
    DivRem4 = 30,

    CastU32 = 31,

    U32And = 32,
    U32AndConstant = 33,

    U32Or = 34,
    U32OrConstant = 35,

    U32Xor = 36,
    U32XorConstant = 37,

    U32ShiftLeft = 38,
    U32ShiftLeftConstantBitDistance = 40,
    U32ShiftLeftConstantValue = 41,

    U32ShiftRight = 42,
    U32ShiftRightConstantBitDistance = 43,
    U32ShiftRightConstantValue = 44,

    CalculateMerkleRoot = 45,

    GetUserId = 46,
    GetContractId = 47,
    GetCheckpointId = 48,
    GetNonce = 49,
    GetUserPublicKeyHash = 50,
    GetStateQueryResult = 51,
    GetStateQueryResultSingle = 52,

    GetStateCommandResultHash = 53,
    GetStateCommandResultSingle = 54,
    GetStateCommandResultArray = 55,

    UnaryInverse = 64,
    UnaryNegative = 65,
}

impl From<u16> for OpType {
    fn from(value: u16) -> Self {
        match value {
            0 => OpType::InputTarget,

            1 => OpType::Constant,
            2 => OpType::ConstantTrue,
            3 => OpType::ConstantFalse,
            4 => OpType::Add,
            5 => OpType::Sub,
            6 => OpType::Mul,
            7 => OpType::Div,
            8 => OpType::BoolNot,
            9 => OpType::BoolAnd,
            10 => OpType::BoolOr,
            11 => OpType::Xor,
            12 => OpType::Nor,
            13 => OpType::Eq,
            14 => OpType::Lte,
            15 => OpType::Gte,
            16 => OpType::Gt,
            17 => OpType::Lt,
            18 => OpType::SplitBits,
            19 => OpType::SumBits,
            20 => OpType::TargetAt,
            21 => OpType::HashNoPad,
            22 => OpType::HashPad,
            23 => OpType::Select,
            24 => OpType::Exp,
            25 => OpType::ExpConstantPower,
            26 => OpType::ExpConstantBase,
            27 => OpType::Mod,
            28 => OpType::ModConstantDividend,
            29 => OpType::ModConstantDivisor,
            30 => OpType::DivRem4,
            31 => OpType::CastU32,
            32 => OpType::U32And,
            33 => OpType::U32AndConstant,
            34 => OpType::U32Or,
            35 => OpType::U32OrConstant,
            36 => OpType::U32Xor,
            37 => OpType::U32XorConstant,
            38 => OpType::U32ShiftLeft,
            40 => OpType::U32ShiftLeftConstantBitDistance,
            41 => OpType::U32ShiftLeftConstantValue,
            42 => OpType::U32ShiftRight,
            43 => OpType::U32ShiftRightConstantBitDistance,
            44 => OpType::U32ShiftRightConstantValue,
            45 => OpType::CalculateMerkleRoot,
            46 => OpType::GetUserId,
            47 => OpType::GetContractId,
            48 => OpType::GetCheckpointId,
            49 => OpType::GetNonce,
            50 => OpType::GetUserPublicKeyHash,
            51 => OpType::GetStateQueryResult,
            52 => OpType::GetStateQueryResultSingle,
            53 => OpType::GetStateCommandResultHash,
            54 => OpType::GetStateCommandResultSingle,
            55 => OpType::GetStateCommandResultArray,
            64 => OpType::UnaryInverse,
            65 => OpType::UnaryNegative,
            _ => panic!("Unknown OpType: {}", value),
        }
    }
}

impl OpType {
    pub fn get_enc_value(&self) -> u16 {
        *self as u16
    }
    pub fn eval_binary_constant(&self, a: u64, b: u64) -> u64 {
        assert!(a < GoldilocksField::ORDER, "value {} is not a valid Felt", a);
        assert!(b < GoldilocksField::ORDER, "value {} is not a valid Felt", b);
        match self {
            OpType::Add =>(GoldilocksField::from_canonical_u64(a) + GoldilocksField::from_canonical_u64(b)).to_canonical_u64(),
            OpType::Sub => (GoldilocksField::from_canonical_u64(a) - GoldilocksField::from_canonical_u64(b)).to_canonical_u64(),
            OpType::Mul =>  (GoldilocksField::from_canonical_u64(a) * GoldilocksField::from_canonical_u64(b)).to_canonical_u64(),
            OpType::Div =>  (GoldilocksField::from_canonical_u64(a) / GoldilocksField::from_canonical_u64(b)).to_canonical_u64(),
            OpType::Xor => GoldilocksField::from_noncanonical_u64(a^b).0,
            OpType::Eq => if a == b {1} else {0},
            OpType::Lte => if a <= b {1} else {0},
            OpType::Gte => if a >= b {1} else {0},
            OpType::Gt => if a > b {1} else {0},
            OpType::Lt => if a < b {1} else {0},
            OpType::Exp => (GoldilocksField::from_canonical_u64(a).exp_u64(b)).to_canonical_u64(),
            OpType::Mod => a%b,
            OpType::U32And => (a&b)&0xffffffffu64,
            OpType::U32Or => (a|b)&0xffffffffu64,
            OpType::U32Xor => (a^b)&0xffffffffu64,
            OpType::U32ShiftLeft => (a<<b)&0xffffffffu64,
            OpType::U32ShiftRight => (a>>b)&0xffffffffu64,
            OpType::BoolAnd => (a&b)&1,
            OpType::BoolOr => (a|b)&1,
            _ => panic!("OpType::eval_binary_constant not implemented for {:?}", self),
        }
    }
    pub fn eval_unary_constant(&self, a: u64) -> u64 {
        assert!(a < GoldilocksField::ORDER, "value {} is not a valid Felt", a);
        match self {
            OpType::BoolNot => if a == 0 {1} else {0},
            OpType::UnaryInverse => GoldilocksField::from_noncanonical_u64(a).inverse().to_canonical_u64(),
            OpType::UnaryNegative => GoldilocksField::from_noncanonical_u64(a).neg().to_canonical_u64(),
            _ => panic!("OpType::eval_unary_constant not implemented for {:?}", self),
        }
    }
    pub fn get_data_type(&self) -> DPNBuiltInDataType {
        match self {
            OpType::InputTarget => DPNBuiltInDataType::Target,
            OpType::Constant => DPNBuiltInDataType::Target,
            OpType::ConstantTrue => DPNBuiltInDataType::Bool,
            OpType::ConstantFalse => DPNBuiltInDataType::Bool,
            OpType::Add => DPNBuiltInDataType::Target,
            OpType::Sub => DPNBuiltInDataType::Target,
            OpType::Mul => DPNBuiltInDataType::Target,
            OpType::Div => DPNBuiltInDataType::Target,
            OpType::BoolNot => DPNBuiltInDataType::Bool,
            OpType::BoolAnd => DPNBuiltInDataType::Bool,
            OpType::BoolOr => DPNBuiltInDataType::Bool,
            OpType::Xor => DPNBuiltInDataType::Target,
            OpType::Nor => DPNBuiltInDataType::Target,
            OpType::Eq => DPNBuiltInDataType::Bool,
            OpType::Lte => DPNBuiltInDataType::Bool,
            OpType::Gte => DPNBuiltInDataType::Bool,
            OpType::Gt => DPNBuiltInDataType::Bool,
            OpType::Lt => DPNBuiltInDataType::Bool,
            OpType::SplitBits => DPNBuiltInDataType::BoolArray,
            OpType::SumBits => DPNBuiltInDataType::Target,
            OpType::TargetAt => DPNBuiltInDataType::Target,
            OpType::HashNoPad => DPNBuiltInDataType::HashOut,
            OpType::HashPad => DPNBuiltInDataType::HashOut,
            OpType::Select => DPNBuiltInDataType::Target,
            OpType::Exp => DPNBuiltInDataType::Target,
            OpType::ExpConstantPower => DPNBuiltInDataType::Target,
            OpType::ExpConstantBase => DPNBuiltInDataType::Target,
            OpType::Mod => DPNBuiltInDataType::Target,
            OpType::ModConstantDividend => DPNBuiltInDataType::Target,
            OpType::ModConstantDivisor => DPNBuiltInDataType::Target,
            OpType::DivRem4 => DPNBuiltInDataType::Target,
            OpType::CastU32 => DPNBuiltInDataType::U32Target,
            OpType::U32And => DPNBuiltInDataType::U32Target,
            OpType::U32AndConstant => DPNBuiltInDataType::U32Target,
            OpType::U32Or => DPNBuiltInDataType::U32Target,
            OpType::U32OrConstant => DPNBuiltInDataType::U32Target,
            OpType::U32Xor => DPNBuiltInDataType::U32Target,
            OpType::U32XorConstant => DPNBuiltInDataType::U32Target,
            OpType::U32ShiftLeft => DPNBuiltInDataType::U32Target,
            OpType::U32ShiftLeftConstantBitDistance => DPNBuiltInDataType::U32Target,
            OpType::U32ShiftLeftConstantValue => DPNBuiltInDataType::U32Target,
            OpType::U32ShiftRight => DPNBuiltInDataType::U32Target,
            OpType::U32ShiftRightConstantBitDistance => DPNBuiltInDataType::U32Target,
            OpType::U32ShiftRightConstantValue => DPNBuiltInDataType::U32Target,
            OpType::CalculateMerkleRoot => DPNBuiltInDataType::HashOut,
            OpType::GetUserId => DPNBuiltInDataType::Target,
            OpType::GetContractId => DPNBuiltInDataType::Target,
            OpType::GetCheckpointId => DPNBuiltInDataType::Target,
            OpType::GetNonce => DPNBuiltInDataType::Target,
            OpType::GetUserPublicKeyHash => DPNBuiltInDataType::HashOut,
            OpType::GetStateQueryResult => DPNBuiltInDataType::HashOut,
            OpType::GetStateQueryResultSingle => DPNBuiltInDataType::Target,
            OpType::GetStateCommandResultHash => DPNBuiltInDataType::HashOut,
            OpType::GetStateCommandResultSingle => DPNBuiltInDataType::Target,
            OpType::GetStateCommandResultArray => DPNBuiltInDataType::TargetArray,
            OpType::UnaryInverse => DPNBuiltInDataType::Target,
            OpType::UnaryNegative => DPNBuiltInDataType::Target,
        }
    }
    pub fn is_inputless(&self) -> bool{
        match self {
            OpType::ConstantTrue => true,
            OpType::ConstantFalse => true,
            OpType::GetUserId => true,
            OpType::GetContractId => true,
            OpType::GetCheckpointId => true,
            OpType::GetNonce => true,
            OpType::GetUserPublicKeyHash => true,
            _ => false,
        }
    }
    pub fn needs_store(&self) -> bool {
        match self {
            OpType::InputTarget => false,
            OpType::Constant => false,
            OpType::ConstantTrue => false,
            OpType::ConstantFalse => false,
            OpType::GetUserId => false,
            OpType::GetContractId => false,
            OpType::GetCheckpointId => false,
            OpType::GetNonce => false,
            OpType::GetUserPublicKeyHash => false,
            _ => true,
        }
    }
    pub fn has_constant_param(&self) -> bool {
        match self {
            OpType::Constant => true,
            OpType::ConstantTrue => true,
            OpType::ConstantFalse => true,
            OpType::ExpConstantPower => true,
            OpType::ExpConstantBase => true,
            OpType::ModConstantDividend => true,
            OpType::ModConstantDivisor => true,
            OpType::U32AndConstant => true,
            OpType::U32OrConstant => true,
            OpType::U32XorConstant => true,
            OpType::U32ShiftLeftConstantBitDistance => true,
            OpType::U32ShiftLeftConstantValue => true,
            OpType::U32ShiftRightConstantBitDistance => true,
            OpType::U32ShiftRightConstantValue => true,
            OpType::SplitBits => true,
            OpType::GetStateCommandResultSingle => true,
            OpType::GetStateCommandResultArray => true,
            OpType::GetStateCommandResultHash => true,
            
            _ => false,
        }
    }
}

impl std::fmt::Display for OpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match &self {
            OpType::InputTarget => "InputTarget",
            OpType::Constant => "Constant",
            OpType::ConstantTrue => "ConstantTrue",
            OpType::ConstantFalse => "ConstantFalse",
            OpType::Add => "Add",
            OpType::Sub => "Sub",
            OpType::Mul => "Mul",
            OpType::Div => "Div",
            OpType::BoolNot => "BoolNot",
            OpType::BoolAnd => "BoolAnd",
            OpType::BoolOr => "BoolOr",
            OpType::Xor => "Xor",
            OpType::Nor => "Nor",
            OpType::Eq => "Eq",
            OpType::Lte => "Lte",
            OpType::Gte => "Gte",
            OpType::Gt => "Gt",
            OpType::Lt => "Lt",
            OpType::SplitBits => "SplitBits",
            OpType::SumBits => "SumBits",
            OpType::TargetAt => "TargetAt",
            OpType::HashNoPad => "HashNoPad",
            OpType::HashPad => "HashPad",
            OpType::Select => "Select",
            OpType::Exp => "Exp",
            OpType::ExpConstantPower => "ExpConstantPower",
            OpType::ExpConstantBase => "ExpConstantBase",
            OpType::Mod => "Mod",
            OpType::ModConstantDividend => "ModConstantDividend",
            OpType::ModConstantDivisor => "ModConstantDivisor",
            OpType::DivRem4 => "DivRem4",
            OpType::CastU32 => "CastU32",
            OpType::U32And => "U32And",
            OpType::U32AndConstant => "U32AndConstant",
            OpType::U32Or => "U32Or",
            OpType::U32OrConstant => "U32OrConstant",
            OpType::U32Xor => "U32Xor",
            OpType::U32XorConstant => "U32XorConstant",
            OpType::U32ShiftLeft => "U32ShiftLeft",
            OpType::U32ShiftLeftConstantBitDistance => "U32ShiftLeftConstantBitDistance",
            OpType::U32ShiftLeftConstantValue => "U32ShiftLeftConstantValue",
            OpType::U32ShiftRight => "U32ShiftRight",
            OpType::U32ShiftRightConstantBitDistance => "U32ShiftRightConstantBitDistance",
            OpType::U32ShiftRightConstantValue => "U32ShiftRightConstantValue",
            OpType::CalculateMerkleRoot => "CalculateMerkleRoot",
            OpType::GetUserId => "GetUserId",
            OpType::GetContractId => "GetContractId",
            OpType::GetCheckpointId => "GetCheckpointId",
            OpType::GetNonce => "GetNonce",
            OpType::GetUserPublicKeyHash => "GetUserPublicKeyHash",
            OpType::GetStateQueryResult => "GetStateQueryResult",
            OpType::GetStateQueryResultSingle => "GetStateQueryResultSingle",
            OpType::GetStateCommandResultHash => "GetStateCommandResultHash",
            OpType::GetStateCommandResultSingle => "GetStateCommandResultSingle",
            OpType::GetStateCommandResultArray => "GetStateCommandResultArray",
            OpType::UnaryInverse => "UnaryInverse",
            OpType::UnaryNegative => "UnaryNegative",
        };
        write!(f, "OpType::{}", r)
    }
}

#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[repr(u8)]
pub enum DPNBuiltInDataType {
    Target = 0,
    Bool = 1,
    U32Target = 2,
    HashOut = 3,
    HashOut160 = 4,
    TargetArray = 5,
    BoolArray = 6,
    U32TargetArray = 7,
    Unknown = 63,
}

impl std::fmt::Display for DPNBuiltInDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match &self {
            DPNBuiltInDataType::Target => "Target",
            DPNBuiltInDataType::Bool => "Bool",
            DPNBuiltInDataType::HashOut => "HashOut",
            DPNBuiltInDataType::HashOut160 => "HashOut160",
            DPNBuiltInDataType::TargetArray => "TargetArray",
            DPNBuiltInDataType::BoolArray => "BoolArray",
            DPNBuiltInDataType::Unknown => "Unknown",
            DPNBuiltInDataType::U32Target => "U32Target",
            DPNBuiltInDataType::U32TargetArray => "U32TargetArray",
        };
        write!(f, "DPNBuiltInDataType::{}", r)
    }
}

impl From<u64> for DPNBuiltInDataType {
    fn from(x: u64) -> Self {
        match x {
            0 => Self::Target,
            1 => Self::Bool,
            2 => Self::U32Target,
            3 => Self::HashOut,
            4 => Self::HashOut160,
            5 => Self::TargetArray,
            6 => Self::BoolArray,
            7 => Self::U32TargetArray,
            _ => Self::Unknown,
        }
    }
}
impl From<u32> for DPNBuiltInDataType {
    fn from(x: u32) -> Self {
        DPNBuiltInDataType::from(x as u64)
    }
}
impl From<u16> for DPNBuiltInDataType {
    fn from(x: u16) -> Self {
        DPNBuiltInDataType::from(x as u64)
    }
}
impl From<u8> for DPNBuiltInDataType {
    fn from(x: u8) -> Self {
        DPNBuiltInDataType::from(x as u64)
    }
}
