use plonky2::hash::hash_types::RichField;

use crate::dpn::ops::op_types::{DPNBuiltInDataType, DPNOpType};

use super::def::{decode_indexed_op_id, DPNFunctionCircuitDefinition, DPNIndexedVarDef};

pub struct SimpleDPNExecutor<F: RichField> {
    targets: Vec<F>,
    target_arrays: Vec<Vec<F>>,
    hashes: Vec<[F; 4]>,
    hash160s: Vec<[u32; 5]>,
    bools: Vec<bool>,
    bool_arrays: Vec<Vec<bool>>,
    u32s: Vec<u32>,
    u32_arrays: Vec<Vec<u32>>,
}

impl<F: RichField> SimpleDPNExecutor<F> {
    pub fn new() -> Self {
        SimpleDPNExecutor {
            targets: Vec::new(),
            target_arrays: Vec::new(),
            hashes: Vec::new(),
            hash160s: Vec::new(),
            bools: Vec::new(),
            bool_arrays: Vec::new(),
            u32s: Vec::new(),
            u32_arrays: Vec::new(),
        }
    }
    pub fn resolve_bool(&self, id: u64) -> bool {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::Bool => {
                assert!(index < self.bools.len(), "Invalid bool index");
                self.bools[index]
            }
            DPNBuiltInDataType::Target => {
                assert!(index < self.targets.len(), "Invalid target index");

                let uv = self.targets[index].to_canonical_u64();
                assert!(uv == 1 || uv == 0, "Invalid bool value");
                uv == 1
            },
            
            DPNBuiltInDataType::U32Target => {
                assert!(index < self.u32s.len(), "Invalid u32 index");

                let uv = self.u32s[index];
                assert!(uv == 1 || uv == 0, "Invalid bool value");
                uv == 1
            },
            _ => panic!("Invalid data type for bool"),
        }
    }
    pub fn resolve_hash(&self, id: u64) -> [F; 4] {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::HashOut => {
                assert!(index < self.hashes.len(), "Invalid hash index");
                self.hashes[index]
            },
            _ => panic!("Invalid data type for hash"),
        }
    }
    pub fn resolve_hash160(&self, id: u64) -> [u32; 5] {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::HashOut160 => {
                assert!(index < self.hashes.len(), "Invalid hash160 index");
                self.hash160s[index]
            },
            _ => panic!("Invalid data type for hash160"),
        }
    }
    pub fn resolve_target(&self, id: u64) -> F {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::Bool => {
                assert!(index < self.bools.len(), "Invalid bool index");
                if self.bools[index] {
                    F::ONE
                } else {
                    F::ZERO
                }
            },
            DPNBuiltInDataType::Target => {
                assert!(index < self.targets.len(), "Invalid target index");
                self.targets[index]
            },
            
            DPNBuiltInDataType::U32Target => {
                assert!(index < self.u32s.len(), "Invalid u32 index");

                F::from_canonical_u32(self.u32s[index])
            },
            _ => panic!("Invalid data type for target"),
        }

    }
    pub fn resolve_u32(&self, id: u64) -> u32 {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            
            DPNBuiltInDataType::U32Target => {
                assert!(index < self.u32s.len(), "Invalid u32 index");

                self.u32s[index]
            },
            DPNBuiltInDataType::Bool => {
                assert!(index < self.bools.len(), "Invalid bool index");
                if self.bools[index] {
                    1
                } else {
                    0
                }
            },
            DPNBuiltInDataType::Target => {
                assert!(index < self.targets.len(), "Invalid target index");
                self.targets[index].to_canonical_u64() as u32
            },
            _ => panic!("Invalid data type for U32Target"),
        }

    }
    pub fn resolve_target_array(&self, id: u64) -> Vec<F> {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::BoolArray => {
                assert!(index < self.bool_arrays.len(), "Invalid bool array index");
                self.bool_arrays[index].iter().map(|b| if *b { F::ONE } else { F::ZERO }).collect()
            },
            DPNBuiltInDataType::TargetArray => {
                assert!(index < self.target_arrays.len(), "Invalid target array index");
                self.target_arrays[index].clone()
            },
            
            DPNBuiltInDataType::U32TargetArray => {
                assert!(index < self.u32_arrays.len(), "Invalid u32 array index");

                self.u32_arrays[index].iter().map(|b| F::from_canonical_u32(*b)).collect()
            },
            _ => panic!("Invalid data type for target array"),
        }

    }
    pub fn resolve_bool_array(&self, id: u64) -> Vec<bool> {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::BoolArray => {
                assert!(index < self.bool_arrays.len(), "Invalid bool array index");
                self.bool_arrays[index].clone()
            },
            _ => panic!("Invalid data type for bool array"),
        }
    }
    pub fn resolve_u32_array(&self, id: u64) -> Vec<u32> {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::U32TargetArray => {
                assert!(index < self.u32_arrays.len(), "Invalid u32 array index");
                self.u32_arrays[index].clone()
            },
            _ => panic!("Invalid data type for bool array"),
        }
    }


    pub fn process_var_def(&mut self, op: &DPNIndexedVarDef) {
        
        match op.op_type {
            DPNOpType::InputTarget => todo!("this shouldn't ever get called probably"),
            DPNOpType::Constant => self.targets.push(F::from_canonical_u64(op.inputs[0])),
            DPNOpType::ConstantTrue => self.bools.push(true),
            DPNOpType::ConstantFalse => self.bools.push(false),
            DPNOpType::Add => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(left + right);
            },
            DPNOpType::Sub => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(left - right);
            },
            DPNOpType::Mul => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(left * right);
            },
            DPNOpType::Div => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(left / right);
            },
            DPNOpType::BoolNot => {
                let left = self.resolve_bool(op.inputs[0]);
                self.bools.push(!left);
            },
            DPNOpType::BoolAnd =>{
                let left = self.resolve_bool(op.inputs[0]);
                let right = self.resolve_bool(op.inputs[1]);
                self.bools.push(left && right);
            },
            DPNOpType::BoolOr => {
                let left = self.resolve_bool(op.inputs[0]);
                let right = self.resolve_bool(op.inputs[1]);
                self.bools.push(left && right);
            },
            DPNOpType::Xor => todo!(),
            DPNOpType::Nor => todo!(),
            DPNOpType::Eq => todo!(),
            DPNOpType::Lte => todo!(),
            DPNOpType::Gte => todo!(),
            DPNOpType::Gt => todo!(),
            DPNOpType::Lt => todo!(),
            DPNOpType::SplitBits => todo!(),
            DPNOpType::SumBits => todo!(),
            DPNOpType::TargetAt => todo!(),
            DPNOpType::HashNoPad => todo!(),
            DPNOpType::HashPad => todo!(),
            DPNOpType::Select => todo!(),
            DPNOpType::Exp => todo!(),
            DPNOpType::ExpConstantPower => todo!(),
            DPNOpType::ExpConstantBase => todo!(),
            DPNOpType::Mod => todo!(),
            DPNOpType::ModConstantDividend => todo!(),
            DPNOpType::ModConstantDivisor => todo!(),
            DPNOpType::DivRem4 => todo!(),
            DPNOpType::CastU32 => todo!(),
            DPNOpType::U32And => todo!(),
            DPNOpType::U32AndConstant => todo!(),
            DPNOpType::U32Or => todo!(),
            DPNOpType::U32OrConstant => todo!(),
            DPNOpType::U32Xor => todo!(),
            DPNOpType::U32XorConstant => todo!(),
            DPNOpType::U32ShiftLeft => todo!(),
            DPNOpType::U32ShiftLeftConstantBitDistance => todo!(),
            DPNOpType::U32ShiftLeftConstantValue => todo!(),
            DPNOpType::U32ShiftRight => todo!(),
            DPNOpType::U32ShiftRightConstantBitDistance => todo!(),
            DPNOpType::U32ShiftRightConstantValue => todo!(),
            DPNOpType::CalculateMerkleRoot => todo!(),
            DPNOpType::GetUserId => todo!(),
            DPNOpType::GetContractId => todo!(),
            DPNOpType::GetCheckpointId => todo!(),
            DPNOpType::GetNonce => todo!(),
            DPNOpType::GetUserPublicKeyHash => todo!(),
            DPNOpType::GetStateQueryResult => todo!(),
            DPNOpType::GetStateQueryResultSingle => todo!(),
            DPNOpType::GetStateCommandResultHash => todo!(),
            DPNOpType::GetStateCommandResultSingle => todo!(),
            DPNOpType::GetStateCommandResultArray => todo!(),
            DPNOpType::UnaryInverse => todo!(),
            DPNOpType::UnaryNegative => todo!(),
        }
        
    }
}