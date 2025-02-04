use plonky2::hash::hash_types::RichField;

use crate::dpn::ops::op_types::{decode_indexed_op_id, DPNBuiltInDataType, DPNIndexedVarDef, DPNOpType};


pub struct SimpleDPNExecutor<F: RichField> {
    pub targets: Vec<F>,
    pub target_arrays: Vec<Vec<F>>,
    pub hashes: Vec<[F; 4]>,
    pub hash160s: Vec<[u32; 5]>,
    pub bools: Vec<bool>,
    pub bool_arrays: Vec<Vec<bool>>,
    pub u32s: Vec<u32>,
    pub u32_arrays: Vec<Vec<u32>>,
    pub user_id: F,
    pub contract_id: F,
    pub checkpoint_id: F,
    pub nonce: F,
    pub inputs: Vec<F>,
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
            user_id: F::ZERO,
            contract_id: F::ZERO,
            checkpoint_id: F::ZERO,
            nonce: F::ZERO,
            inputs: Vec::new(),
            
        }
    }
    pub fn new_with_contract_ctx(inputs: Vec<F>, user_id: F, contract_id: F, checkpoint_id: F, nonce: F) -> Self {
        SimpleDPNExecutor {
            targets: Vec::new(),
            target_arrays: Vec::new(),
            hashes: Vec::new(),
            hash160s: Vec::new(),
            bools: Vec::new(),
            bool_arrays: Vec::new(),
            u32s: Vec::new(),
            u32_arrays: Vec::new(),
            user_id,
            contract_id,
            checkpoint_id,
            nonce,
            inputs,
            
        }
    }
    pub fn push_external_target(&mut self, target: F) {
        self.targets.push(target);
    }
    pub fn push_external_target_array(&mut self, target: Vec<F>) {
        self.target_arrays.push(target);
    }
    pub fn push_external_hash(&mut self, target: [F; 4]) {
        self.hashes.push(target);
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
    pub fn resolve_target_array_ref(&self, id: u64, index_id: u64) -> F {
        let (t, index) = decode_indexed_op_id(id);
        //println!("array_data_type: {:?}, arr_index: {}", t, index);
        //println!("in_array_index_target_id: {}",index_id);

        let ind_real = self.resolve_target(index_id);
        //println!("in_array_index_target_id: {} (equals {})",index_id, ind_real.to_canonical_u64());
        
        match t {
            DPNBuiltInDataType::HashOut => {
                assert!(ind_real.to_canonical_u64() < 4, "Invalid index in hash");
                self.hashes[index][ind_real.to_canonical_u64() as usize]
            },
            DPNBuiltInDataType::HashOut160 => {
                assert!(ind_real.to_canonical_u64() < 5, "Invalid index in hash160");
                F::from_canonical_u32(self.hash160s[index][ind_real.to_canonical_u64() as usize])
            },
            DPNBuiltInDataType::BoolArray => {
                assert!(index < self.bool_arrays.len(), "Invalid bool array index");
                if self.bool_arrays[index][ind_real.to_canonical_u64() as usize] {
                    F::ONE
                } else {
                    F::ZERO
                }
            },
            DPNBuiltInDataType::TargetArray => {
                assert!(index < self.target_arrays.len(), "Invalid target array index");
                self.target_arrays[index][ind_real.to_canonical_u64() as usize]
            },
            
            DPNBuiltInDataType::U32TargetArray => {
                assert!(index < self.u32_arrays.len(), "Invalid u32 array index");
                F::from_canonical_u32(self.u32_arrays[index][ind_real.to_canonical_u64() as usize])
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

    fn print_current_op(&self, op: &DPNIndexedVarDef) {
        match op.data_type {
            DPNBuiltInDataType::Target => println!("d_target[{}] -> {:?}", self.targets.len(), op),
            DPNBuiltInDataType::Bool => println!("d_bool[{}] -> {:?}", self.u32_arrays.len(), op),
            DPNBuiltInDataType::U32Target => println!("d_u32[{}] -> {:?}", self.u32s.len(), op),
            DPNBuiltInDataType::HashOut => println!("d_hashout[{}] -> {:?}", self.hashes.len(), op),
            DPNBuiltInDataType::HashOut160 => println!("d_hash160[{}] -> {:?}", self.hash160s.len(), op),
            DPNBuiltInDataType::TargetArray => println!("d_target_array[{}] -> {:?}", self.target_arrays.len(), op),
            DPNBuiltInDataType::BoolArray => println!("d_bool_array[{}] -> {:?}", self.bool_arrays.len(), op),
            DPNBuiltInDataType::U32TargetArray => println!("d_u32_array[{}] -> {:?}", self.u32_arrays.len(), op),
            DPNBuiltInDataType::Unknown => println!("d_unknown: {:?}", op),
        }
    }


    pub fn process_var_def(&mut self, op: &DPNIndexedVarDef) {
        //self.print_current_op(op);
        
        match op.op_type {
            //DPNOpType::InputTarget => todo!("this shouldn't ever get called probably"),
            DPNOpType::InputTarget => {
                let index = op.inputs[0] as usize;
                if index >= self.inputs.len() {
                    panic!("Invalid input index");
                }else{
                    self.targets.push(self.inputs[index]);
                }
            },
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
            DPNOpType::Eq => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.bools.push(left == right);
            },
            DPNOpType::Lte => {
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                self.bools.push(left <= right);
            },
            DPNOpType::Gte => {
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                self.bools.push(left >= right);
            },
            DPNOpType::Gt => {
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                self.bools.push(left > right);
            },
            DPNOpType::Lt => {
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                self.bools.push(left < right);
            },
            DPNOpType::SplitBits => todo!(),
            DPNOpType::SumBits => todo!(),
            DPNOpType::TargetAt => {
                let r = self.resolve_target_array_ref(op.inputs[0], op.inputs[1]);
                self.targets.push(r);
            },
            DPNOpType::HashNoPad => todo!(),
            DPNOpType::HashPad => todo!(),
            DPNOpType::Select => {
                let condition = self.resolve_target(op.inputs[0]);
                let result = if condition.to_canonical_u64() != 0 {
                    self.resolve_target(op.inputs[1])
                } else {
                    self.resolve_target(op.inputs[2])
                };
                self.targets.push(result);
            },
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
            DPNOpType::GetUserId => self.targets.push(self.user_id),
            DPNOpType::GetContractId => self.targets.push(self.contract_id),
            DPNOpType::GetCheckpointId => self.targets.push(self.checkpoint_id),
            DPNOpType::GetNonce => self.targets.push(self.nonce),
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