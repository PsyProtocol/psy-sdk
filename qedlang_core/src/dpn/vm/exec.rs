use plonky2::{hash::{hash_types::RichField, poseidon::PoseidonHash}, plonk::config::Hasher};

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
    pub user_public_key: [F; 4],
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
            user_public_key: [F::ZERO; 4],
            nonce: F::ZERO,
            inputs: Vec::new(),
            
        }
    }
    pub fn new_with_contract_ctx(inputs: Vec<F>, user_id: F, contract_id: F, checkpoint_id: F, nonce: F, user_public_key: [F; 4]) -> Self {
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
            user_public_key,
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
    pub fn resolve_targets(&self, id: &[u64]) -> Vec<F> {
        id.iter().map(|id| self.resolve_target(*id)).collect()
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
                self.bools.push(left || right);
            },
            DPNOpType::Xor => {
                let left = self.resolve_bool(op.inputs[0]);
                let right = self.resolve_bool(op.inputs[1]);
                self.bools.push(left ^ right);
            },
            DPNOpType::Nor => {
                let left = self.resolve_bool(op.inputs[0]);
                let right = self.resolve_bool(op.inputs[1]);
                self.bools.push(!(left || right));
            },
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
            DPNOpType::SplitBits => {
                let target = self.resolve_target(op.inputs[1]);
                let num_bits = op.inputs[0];
                assert!(num_bits <= 64, "SplitBits: num_bits must be less than 64");
                
                let actual_target_bits = 64 - target.to_canonical_u64().leading_zeros();
                assert!(actual_target_bits <= num_bits as u32, "SplitBits: target bits must be less than num_bits");
                
                self.bool_arrays.push(split_bits(target.to_canonical_u64(), num_bits));
            },
            DPNOpType::SumBits => {
                assert!(op.inputs.len() <= 64, "Sumbits: can only sum at most 64 bits");
                let sum = op.inputs.iter().enumerate().map(|(i, &input)| self.resolve_bool(input) as u64 * (1 << i)).sum::<u64>();
         
                assert!(sum <= F::ORDER, "SumBits: sum must be less than field order");
                
                self.targets.push(F::from_canonical_u64(sum));
            },
            DPNOpType::TargetAt => {
                let r = self.resolve_target_array_ref(op.inputs[0], op.inputs[1]);
                self.targets.push(r);
            },
            DPNOpType::HashNoPad => {
                let values = self.resolve_targets(&op.inputs);
                self.hashes.push(PoseidonHash::hash_no_pad(&values).elements);
            },
            DPNOpType::HashTwoToOne => {
                // Expecting 8 inputs: 4 for left hash, 4 for right hash
                assert_eq!(op.inputs.len(), 8, "HashTwoToOne requires exactly 8 inputs");
                let left = [
                    self.resolve_target(op.inputs[0]),
                    self.resolve_target(op.inputs[1]),
                    self.resolve_target(op.inputs[2]),
                    self.resolve_target(op.inputs[3]),
                ];
                let right = [
                    self.resolve_target(op.inputs[4]),
                    self.resolve_target(op.inputs[5]),
                    self.resolve_target(op.inputs[6]),
                    self.resolve_target(op.inputs[7]),
                ];
                let left_hash = plonky2::hash::hash_types::HashOut { elements: left };
                let right_hash = plonky2::hash::hash_types::HashOut { elements: right };
                let result = PoseidonHash::two_to_one(left_hash, right_hash);
                self.hashes.push(result.elements);
            },
            DPNOpType::HashPad => unimplemented!(),
            DPNOpType::Select => {
                let condition = self.resolve_target(op.inputs[0]);
                let result = if condition.to_canonical_u64() != 0 {
                    self.resolve_target(op.inputs[1])
                } else {
                    self.resolve_target(op.inputs[2])
                };
                self.targets.push(result);
            },
            DPNOpType::Exp => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(left.exp_u64(right.to_canonical_u64()));
            },
            DPNOpType::ExpConstantPower => {
                let left = self.resolve_target(op.inputs[0]);
                let (_optype, right) = decode_indexed_op_id(op.inputs[1]);
                self.targets.push(left.exp_u64(right as u64))
            },
            DPNOpType::ExpConstantBase => {
                let (_optype, left) = decode_indexed_op_id(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(F::from_noncanonical_u64(left as u64).exp_u64(right.to_canonical_u64()))
            },
            DPNOpType::Mod => {
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                assert!(right != 0, "Mod by zero");
                self.targets.push(F::from_canonical_u64(left % right));
            }
            DPNOpType::ModConstantDividend => {
                let (_optype, left) = decode_indexed_op_id(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                assert!(right != 0, "Mod by zero");
                self.targets.push(F::from_canonical_u64((left as u64) % right));
            },
            DPNOpType::ModConstantDivisor => {
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let (_optype, right) = decode_indexed_op_id(op.inputs[1]);
                assert!(right != 0, "Mod by zero");
                self.targets.push(F::from_canonical_u64(left % (right as u64)));
            },
            DPNOpType::DivRem4 => {
                let dividend = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let quotient = F::from_noncanonical_u64(dividend >> 2);
                let remainder = F::from_noncanonical_u64(dividend & 3);
                self.target_arrays.push(vec![quotient, remainder]);
            },
            DPNOpType::CastU32 => {
                let (t, index) = decode_indexed_op_id(op.inputs[0]);
                let value = match t {
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
                        let value = self.targets[index].to_canonical_u64();
                        assert!(value <= 0xffffffffu64, "Invalid u32 value");
                        (value & 0xffffffffu64) as u32
                    },
                    _ => panic!("Invalid data type for U32Target"),
                };
                self.u32s.push(value);
            }
            DPNOpType::U32And => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(left & right);
            },
            DPNOpType::U32AndConstant => {
                let left = self.resolve_u32(op.inputs[0]);
                let (_optype, right) = decode_indexed_op_id(op.inputs[1]);
                self.u32s.push(left & (right as u32));
            },
            DPNOpType::U32Or => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(left | right);
            },
            DPNOpType::U32OrConstant => {
                let left = self.resolve_u32(op.inputs[0]);
                let (_optype, right) = decode_indexed_op_id(op.inputs[1]);
                self.u32s.push(left | (right as u32));
            },
            DPNOpType::U32Xor => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(left ^ right);
            },
            DPNOpType::U32XorConstant => {
                let left = self.resolve_u32(op.inputs[0]);
                let (_optype, right) = decode_indexed_op_id(op.inputs[1]);
                self.u32s.push(left ^ (right as u32));
            },
            DPNOpType::U32ShiftLeft => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                if right >= 32 {
                    self.u32s.push(0u32);
                } else {
                    self.u32s.push(left << right);
                }
            },
            DPNOpType::U32ShiftLeftConstantBitDistance => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                if right >= 32 {
                    self.u32s.push(0u32);
                } else {
                    self.u32s.push(left << right);
                }
            },
            DPNOpType::U32ShiftLeftConstantValue => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                if right >= 32 {
                    self.u32s.push(0u32);
                } else {
                    self.u32s.push(left << right);
                }
            },
            DPNOpType::U32ShiftRight => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                if right >= 32 {
                    self.u32s.push(0u32);
                } else {
                    self.u32s.push(left >> right);
                }
            },
            DPNOpType::U32ShiftRightConstantBitDistance => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                if right >= 32 {
                    self.u32s.push(0u32);
                } else {
                    self.u32s.push(left >> right);
                }
            },
            DPNOpType::U32ShiftRightConstantValue => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                if right >= 32 {
                    self.u32s.push(0u32);
                } else {
                    self.u32s.push(left >> right);
                }
            },
            DPNOpType::CalculateMerkleRoot => unimplemented!(),
            DPNOpType::GetUserId => self.targets.push(self.user_id),
            DPNOpType::GetContractId => self.targets.push(self.contract_id),
            DPNOpType::GetCheckpointId => self.targets.push(self.checkpoint_id),
            DPNOpType::GetNonce => self.targets.push(self.nonce),
            DPNOpType::GetUserPublicKeyHash => self.hashes.push(self.user_public_key),

            // GetStateQueryResult is deprecated, use GetStateCommandResult instead
            DPNOpType::GetStateQueryResult => unimplemented!("deprecated"),
            DPNOpType::GetStateQueryResultSingle => unimplemented!("deprecated"),

            DPNOpType::GetStateCommandResultHash => unreachable!(),
            DPNOpType::GetStateCommandResultSingle => unreachable!(),
            DPNOpType::GetStateCommandResultArray => unreachable!(),
            DPNOpType::UnaryInverse => {
                let left = self.resolve_target(op.inputs[0]);
                assert_ne!(left, F::ZERO, "Cannot inverse zero");
                self.targets.push(left.inverse());
            },
            DPNOpType::UnaryNegative => {
                let left = self.resolve_target(op.inputs[0]);
                self.targets.push(left.neg());
            },
            DPNOpType::U32InputTarget => {
                let index = op.inputs[0] as usize;
                if index >= self.inputs.len() {
                    panic!("Invalid input index");
                }else{
                    assert!(self.inputs[index].to_canonical_u64() <= 0xffffffffu64, "Invalid u32 input[{:?}]", index);
                    self.u32s.push(self.inputs[index].to_canonical_u64() as u32);

                }
            },
            DPNOpType::ConstantU32 => {
                assert!(op.inputs[0] <= 0xffffffffu64, "constant u32 value too large");
                self.u32s.push(op.inputs[0] as u32);
            },
            DPNOpType::U32Add => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                assert!(left as u64 + right as u64 <= 0xffffffffu64, "u32 add value too large");
                self.u32s.push(left + right);
            }
            DPNOpType::U32Sub => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                assert!(left > right, "u32 sub value too low");
                self.u32s.push(left - right);
            }
            DPNOpType::U32Mul => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                assert!(left as u64 * right as u64 <= 0xffffffffu64, "u32 mul value too large");
                self.u32s.push(left * right);
            }
            DPNOpType::U32Div => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                assert!(right != 0, "u32 div by zero");
                self.u32s.push(left / right);
            }
            DPNOpType::CastFelt => {
                let (t, index) = decode_indexed_op_id(op.inputs[0]);
                let value = match t {
                    DPNBuiltInDataType::U32Target => {
                        assert!(index < self.u32s.len(), "Invalid u32 index");

                        self.u32s[index] as u64
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
                        self.targets[index].to_canonical_u64()
                    },
                    _ => panic!("Invalid data type for Target"),
                };
                self.targets.push(F::from_canonical_u64(value));
            }
            DPNOpType::CastBool => {
                let (t, index) = decode_indexed_op_id(op.inputs[0]);
                let value = match t {
                    DPNBuiltInDataType::U32Target => {
                        assert!(index < self.u32s.len(), "Invalid u32 index");
                        assert!(self.u32s[index]<= 1, "Invalid bool value");
                        self.u32s[index] != 0
                    },
                    DPNBuiltInDataType::Bool => {
                        assert!(index < self.bools.len(), "Invalid bool index");
                        self.bools[index] 
                    },
                    DPNBuiltInDataType::Target => {
                        assert!(index < self.targets.len(), "Invalid target index");
                        assert!(self.targets[index].to_canonical_u64()<= 1, "Invalid bool value");
                        self.targets[index].to_canonical_u64() != 0
                    },
                    _ => panic!("Invalid data type for Target"),
                };
                self.bools.push(value);
            }
            DPNOpType::BoolInputTarget => {
                let index = op.inputs[0] as usize;
                if index >= self.inputs.len() {
                    panic!("Invalid input index");
                }else{
                    assert!(self.inputs[index].to_canonical_u64() <= 1, "Invalid bool input[{:?}]", index);
                    self.bools.push(self.inputs[index].to_canonical_u64() != 0);
                }
            }
            DPNOpType::U32Mod => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                assert!(right!= 0, "u32 mod by zero");
                self.u32s.push(left % right);
            }
            DPNOpType::U32Exp => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                let res  = left.exp_u64(right.to_canonical_u64()).to_canonical_u64();
                assert!( res <= 0xffffffffu64, "u32 exp value too large");
                self.u32s.push(res as u32);
            }
            DPNOpType::Secp256k1Verify => {
                // 8 + 8 + 8 + 8 + 8 = 40
                use k256::ecdsa::Signature;
                use k256::ecdsa::signature::hazmat::PrehashVerifier;
                let inputs = self.resolve_targets(&op.inputs);
                assert!(inputs.len() == 36, "Secp256k1Verify input length must be 36");
                let pk_u32 = inputs[0..16]
                    .to_vec()
                    .iter()
                    .map(|k| {
                        assert!(k.to_canonical_u64() < 0xffffffffu64, "secp pk.x must be [u32; 16]");
                        k.to_canonical_u64() as u32
                    })
                    .collect::<Vec<u32>>();
                let pk_x_bytes = pk_u32[0..8]
                    .iter()
                    .flat_map(|&num| num.to_le_bytes())
                    .rev()
                    .collect::<Vec<_>>();
                let pk_y_bytes = pk_u32[8..16]
                    .iter()
                    .flat_map(|&num| num.to_le_bytes())
                    .rev()
                    .collect::<Vec<_>>();
                let mut pk_sec1_bytes = vec![0x04];
                pk_sec1_bytes.extend(pk_x_bytes);
                pk_sec1_bytes.extend(pk_y_bytes);
                let vk = k256::ecdsa::VerifyingKey::from_sec1_bytes(&pk_sec1_bytes)
                    .expect("secp pk must be valid");
                let signature_u32 = inputs[16..32]
                    .to_vec()
                    .iter()
                    .map(|k| {
                        assert!(k.to_canonical_u64() < 0xffffffffu64, "secp signature must be [u32; 16]");
                        k.to_canonical_u64() as u32
                    })
                    .collect::<Vec<u32>>();

                let signature_r_bytes = signature_u32[0..8]
                    .iter()
                    .flat_map(|&num| num.to_le_bytes())
                    .rev()
                    .collect::<Vec<_>>();
                let signature_s_bytes = signature_u32[8..16]
                    .iter()
                    .flat_map(|&num| num.to_le_bytes())
                    .rev()
                    .collect::<Vec<_>>();

                let signature_bytes = signature_r_bytes
                    .iter()
                    .chain(signature_s_bytes.iter())
                    .cloned()
                    .collect::<Vec<_>>();

                let signature =
                    Signature::from_slice(&signature_bytes).expect("secp signature must be valid");

                let msg_bytes = inputs[32..36]
                    .iter()
                    .flat_map(|&num| num.to_canonical_u64().to_le_bytes())
                    .rev()
                    .collect::<Vec<_>>();

                match vk.verify_prehash(&msg_bytes, &signature) {
                    Ok(_) => self.bools.push(true),
                    Err(_) => self.bools.push(false),
                }
            }
        }
        
    }
}

fn split_bits(x: u64, num_bits: u64) -> Vec<bool> {
    let mut result = vec![false; num_bits as usize];
    for i in 0..num_bits {
        result[i as usize] = ((x >> i) & 1) != 0;
    }
    result
}