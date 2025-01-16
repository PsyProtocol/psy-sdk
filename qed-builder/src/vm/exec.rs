use plonky2::hash::hash_types::RichField;

use crate::{ops::DPNBuiltInDataType, OpType};

use super::def::{decode_indexed_op_id, DPNFunctionCircuitDefinition, DPNIndexedVarDef};

pub struct SimpleDPNExecutor<F: RichField> {
    pub targets: Vec<F>,
    pub target_arrays: Vec<Vec<F>>,
    pub hashes: Vec<[F; 4]>,
    pub hash160s: Vec<[u32; 5]>,
    pub bools: Vec<bool>,
    pub bool_arrays: Vec<Vec<bool>>,
    pub u32s: Vec<u32>,
    pub u32_arrays: Vec<Vec<u32>>,
    pub definition_index: usize,
    pub ctx: IExtendedExecutionContext<F>,
}

fn split_bits(x: u64, num_bits: u64) -> Vec<bool> {
    let mut result = vec![false; num_bits as usize];
    for i in 0..num_bits {
        result[i as usize] = ((x >> i) & 1) != 0;
    }
    result
}
fn sum_bits(bits: &[bool]) -> u64 {
    assert!(bits.len() <= 64, "cannot sum more than 64 bits");
    let result = bits.iter().fold(0, |acc, x| acc + (*x as u64));
    result
}

impl<F: RichField> SimpleDPNExecutor<F> {
    // pub fn new() -> Self {
    //     SimpleDPNExecutor {
    //         targets: Vec::new(),
    //         target_arrays: Vec::new(),
    //         hashes: Vec::new(),
    //         hash160s: Vec::new(),
    //         bools: Vec::new(),
    //         bool_arrays: Vec::new(),
    //         u32s: Vec::new(),
    //         u32_arrays: Vec::new(),
    //         definition_index: 0,
    //         ctx: IExtendedExecutionContext<F>::new(),
    //     }
    // }
    pub fn new_with_ctx(ctx: IExtendedExecutionContext<F>) -> Self {
        SimpleDPNExecutor {
            targets: Vec::new(),
            target_arrays: Vec::new(),
            hashes: Vec::new(),
            hash160s: Vec::new(),
            bools: Vec::new(),
            bool_arrays: Vec::new(),
            u32s: Vec::new(),
            u32_arrays: Vec::new(),
            definition_index: 0,
            ctx,
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
                // assert!(uv == 1 || uv == 0, "Invalid bool value");
                uv >= 1
            }

            DPNBuiltInDataType::U32Target => {
                assert!(index < self.u32s.len(), "Invalid u32 index");

                let uv = self.u32s[index];
                assert!(uv == 1 || uv == 0, "Invalid bool value");
                uv == 1
            }
            _ => panic!("Invalid data type for bool"),
        }
    }
    pub fn resolve_hash(&self, id: u64) -> [F; 4] {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::HashOut => {
                assert!(index < self.hashes.len(), "Invalid hash index");
                self.hashes[index]
            }
            _ => panic!("Invalid data type for hash"),
        }
    }
    pub fn resolve_hash160(&self, id: u64) -> [u32; 5] {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::HashOut160 => {
                assert!(index < self.hashes.len(), "Invalid hash160 index");
                self.hash160s[index]
            }
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
            }
            DPNBuiltInDataType::Target => {
                assert!(index < self.targets.len(), "Invalid target index");
                self.targets[index]
            }

            DPNBuiltInDataType::U32Target => {
                assert!(index < self.u32s.len(), "Invalid u32 index");

                F::from_canonical_u32(self.u32s[index])
            }
            _ => panic!("Invalid data type for target"),
        }
    }
    pub fn resolve_u32(&self, id: u64) -> u32 {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::U32Target => {
                assert!(index < self.u32s.len(), "Invalid u32 index");

                self.u32s[index]
            }
            DPNBuiltInDataType::Bool => {
                assert!(index < self.bools.len(), "Invalid bool index");
                if self.bools[index] {
                    1
                } else {
                    0
                }
            }
            DPNBuiltInDataType::Target => {
                assert!(index < self.targets.len(), "Invalid target index");
                self.targets[index].to_canonical_u64() as u32
            }
            _ => panic!("Invalid data type for U32Target"),
        }
    }
    pub fn resolve_target_array(&self, id: u64) -> Vec<F> {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::BoolArray => {
                assert!(index < self.bool_arrays.len(), "Invalid bool array index");
                self.bool_arrays[index]
                    .iter()
                    .map(|b| if *b { F::ONE } else { F::ZERO })
                    .collect()
            }
            DPNBuiltInDataType::TargetArray => {
                assert!(
                    index < self.target_arrays.len(),
                    "Invalid target array index"
                );
                self.target_arrays[index].clone()
            }

            DPNBuiltInDataType::U32TargetArray => {
                assert!(index < self.u32_arrays.len(), "Invalid u32 array index");

                self.u32_arrays[index]
                    .iter()
                    .map(|b| F::from_canonical_u32(*b))
                    .collect()
            }
            _ => panic!("Invalid data type for target array"),
        }
    }
    pub fn resolve_bool_array(&self, id: u64) -> Vec<bool> {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::BoolArray => {
                assert!(index < self.bool_arrays.len(), "Invalid bool array index");
                self.bool_arrays[index].clone()
            }
            _ => panic!("Invalid data type for bool array"),
        }
    }
    pub fn resolve_u32_array(&self, id: u64) -> Vec<u32> {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::U32TargetArray => {
                assert!(index < self.u32_arrays.len(), "Invalid u32 array index");
                self.u32_arrays[index].clone()
            }
            _ => panic!("Invalid data type for bool array"),
        }
    }

    pub fn process_var_def(&mut self, op: &DPNIndexedVarDef) {
        match op.data_type {
            DPNBuiltInDataType::Target => {
                assert_eq!(
                    self.targets.len(),
                    op.index,
                    "index mismatch for Target, expected index {:?}, got index {:?}",
                    self.targets.len(),
                    op.index
                );
            }
            DPNBuiltInDataType::Bool => assert_eq!(
                self.bools.len(),
                op.index,
                "index mismatch for Bool, expected index {:?}, got index {:?}",
                self.bools.len(),
                op.index
            ),
            DPNBuiltInDataType::U32Target => assert_eq!(
                self.u32s.len(),
                op.index,
                "index mismatch for U32Target, expected index {:?}, got index {:?}",
                self.u32s.len(),
                op.index
            ),
            DPNBuiltInDataType::HashOut => assert_eq!(
                self.hashes.len(),
                op.index,
                "index mismatch for HashOut, expected index {:?}, got index {:?}",
                self.hashes.len(),
                op.index
            ),
            DPNBuiltInDataType::HashOut160 => assert_eq!(
                self.hash160s.len(),
                op.index,
                "index mismatch for HashOut160, expected index {:?}, got index {:?}",
                self.hash160s.len(),
                op.index
            ),
            DPNBuiltInDataType::TargetArray => assert_eq!(
                self.target_arrays.len(),
                op.index,
                "index mismatch for TargetArray, expected index {:?}, got index {:?}",
                self.target_arrays.len(),
                op.index
            ),
            DPNBuiltInDataType::BoolArray => assert_eq!(
                self.bool_arrays.len(),
                op.index,
                "index mismatch for BoolArray, expected index {:?}, got index {:?}",
                self.bool_arrays.len(),
                op.index
            ),
            DPNBuiltInDataType::U32TargetArray => assert_eq!(
                self.u32_arrays.len(),
                op.index,
                "index mismatch for U32TargetArray, expected index {:?}, got index {:?}",
                self.u32_arrays.len(),
                op.index
            ),
            DPNBuiltInDataType::Unknown => panic!("unknown built in data type!"),
        }
        match op.op_type {
            OpType::InputTarget => {
                assert!(
                    op.index < self.ctx.exec_inputs.len(), "tried to access out of bounds input with index {:?}, but only {:?} inputs were provided", op.index, self.ctx.exec_inputs.len());
                self.targets
                    .push(F::from_canonical_u64(self.ctx.exec_inputs[op.index]));
            }
            OpType::Constant => {
                if op.inputs.len() == 1 {
                    self.targets
                        .push(F::from_canonical_u64(op.inputs[0] & 0xffffffff));
                } else if op.inputs.len() == 2 {
                    self.targets.push(F::from_canonical_u64(
                        ((op.inputs[1] & 0xffffffff) << 32) | (op.inputs[0] & 0xffffffff),
                    ));
                } else {
                    panic!(
                        "expected 1 or 2 inputs for constant, got {} inputs",
                        op.inputs.len()
                    );
                }
            }
            OpType::ConstantTrue => self.bools.push(true),
            OpType::ConstantFalse => self.bools.push(false),
            OpType::Add => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(left + right);
            }
            OpType::Sub => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(left - right);
            }
            OpType::Mul => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(left * right);
            }
            OpType::Div => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(left / right);
            }
            OpType::BoolNot => {
                let left = self.resolve_bool(op.inputs[0]);
                self.bools.push(!left);
            }
            OpType::BoolAnd => {
                let left = self.resolve_bool(op.inputs[0]);
                let right = self.resolve_bool(op.inputs[1]);
                self.bools.push(left && right);
            }
            OpType::BoolOr => {
                let left = self.resolve_bool(op.inputs[0]);
                let right = self.resolve_bool(op.inputs[1]);
                self.bools.push(left || right);
            }
            OpType::Xor => {
                let left = self.resolve_bool(op.inputs[0]);
                let right = self.resolve_bool(op.inputs[1]);
                self.bools.push(left ^ right);
            }
            OpType::Nor => {
                let left = self.resolve_bool(op.inputs[0]);
                let right = self.resolve_bool(op.inputs[1]);
                self.bools.push((!left) && (!right));
            }
            OpType::Eq => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.bools.push(left == right);
            }
            OpType::Lte => {
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                self.bools.push(left <= right);
            }
            OpType::Gte => {
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                self.bools.push(left >= right);
            }
            OpType::Gt => {
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                self.bools.push(left > right);
            }
            OpType::Lt => {
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                self.bools.push(left < right);
            }
            OpType::SplitBits => {
                assert!(
                    op.inputs.len() == 2,
                    "expected 2 input for SplitBits op, got {} inputs",
                    op.inputs.len()
                );
                assert!(
                    op.inputs[1] >= 1 && op.inputs[1] <= 64,
                    "number of bits in SplitBits op must be greater tha 1 and less than or equal to 64, got nBits = {}",
                    op.inputs[1]
                );
                let bits = split_bits(
                    self.resolve_target(op.inputs[0]).to_canonical_u64(),
                    op.inputs[1],
                );
                self.bool_arrays.push(bits);
            }
            OpType::SumBits => {
                assert!(
                    op.inputs.len() >= 1,
                    "expected at least 1 input for SumBits op, got {} inputs",
                    op.inputs.len()
                );
                // check?
                let sum = sum_bits(&self.resolve_bool_array(op.inputs[0]));
                self.targets.push(F::from_canonical_u64(sum));
            }
            OpType::TargetAt => {
                assert!(
                    op.inputs.len() == 2,
                    "expected 2 inputs for TargetAt op, got {} inputs",
                    op.inputs.len()
                );
                let index = op.inputs[1] as usize;
                self.targets
                    .push(self.resolve_target_array(op.inputs[0])[index]);
            }
            OpType::HashNoPad => {
                todo!()
            }
            OpType::HashPad => {
                todo!()
            }
            OpType::Select => {
                assert!(
                    op.inputs.len() == 3,
                    "expected 3 inputs for Select op, got {} inputs",
                    op.inputs.len()
                );
                let condition_input_res = self.resolve_bool(op.inputs[0]);
                let is_true_value_res = self.resolve_target(op.inputs[1]);
                let is_false_value_res = self.resolve_target(op.inputs[2]);
                let result = if condition_input_res {
                    is_true_value_res
                } else {
                    is_false_value_res
                };
                self.targets.push(result);
            }
            OpType::Exp => {
                assert!(
                    op.inputs.len() == 2,
                    "expected 2 inputs for Exp op, got {} inputs",
                    op.inputs.len()
                );
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(left.exp_u64(right.to_canonical_u64()));
            }
            OpType::ExpConstantPower => {
                assert!(
                    op.inputs.len() == 2,
                    "expected 2 inputs for ExpConstantPower op, got {} inputs",
                    op.inputs.len()
                );
                let left = self.resolve_target(op.inputs[0]);
                let right = op.inputs[1];
                self.targets.push(left.exp_u64(right));
            }
            OpType::ExpConstantBase => {
                assert!(
                    op.inputs.len() == 2,
                    "expected 2 inputs for ExpConstantBase op, got {} inputs",
                    op.inputs.len()
                );
                let left = F::from_canonical_u64(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(left.exp_u64(right.to_canonical_u64()));
            }
            OpType::Mod => {
                assert!(
                    op.inputs.len() == 2,
                    "expected 2 inputs for Select op, got {} inputs",
                    op.inputs.len()
                );
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                self.targets.push(F::from_canonical_u64(left % right));
            }
            OpType::ModConstantDividend => {
                let left = op.inputs[0];
                let right = self.resolve_target(op.inputs[1]).to_canonical_u64();
                self.targets.push(F::from_canonical_u64(left % right));
            }
            OpType::ModConstantDivisor => {
                let left = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let right = op.inputs[1];
                self.targets.push(F::from_canonical_u64(left % right));
            }
            OpType::DivRem4 => {
                let dividend = self.resolve_target(op.inputs[0]).to_canonical_u64();
                let quotient = F::from_canonical_u64(dividend >> 2);
                let remainder = F::from_canonical_u64(dividend & 3);
                self.target_arrays.push(vec![quotient, remainder]);
            }
            OpType::CastU32 => {
                assert!(
                    op.inputs.len() == 1,
                    "expected 1 input for CastU32 op, got {} inputs",
                    op.inputs.len()
                );
                let value = self.resolve_target(op.inputs[0]).to_canonical_u64();
                self.u32s.push((value & 0xffffffff) as u32);
            }
            OpType::U32And => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(left & right);
            }
            OpType::U32AndConstant => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = (op.inputs[1] & 0xffffffff) as u32;
                self.u32s.push(left & right);
            }
            OpType::U32Or => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(left | right);
            }
            OpType::U32OrConstant => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = (op.inputs[1] & 0xffffffff) as u32;
                self.u32s.push(left | right);
            }
            OpType::U32Xor => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(left ^ right);
            }
            OpType::U32XorConstant => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = (op.inputs[1] & 0xffffffff) as u32;
                self.u32s.push(left ^ right);
            }
            OpType::U32ShiftLeft => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(left << right);
            }
            OpType::U32ShiftLeftConstantBitDistance => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = (op.inputs[1] & 0xffffffff) as u32;
                self.u32s.push(left << right);
            }
            OpType::U32ShiftLeftConstantValue => {
                let left = (op.inputs[0] & 0xffffffff) as u32;
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(left << right);
            }
            OpType::U32ShiftRight => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(left >> right);
            }
            OpType::U32ShiftRightConstantBitDistance => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = (op.inputs[1] & 0xffffffff) as u32;
                self.u32s.push(left >> right);
            }
            OpType::U32ShiftRightConstantValue => {
                let left = (op.inputs[0] & 0xffffffff) as u32;
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(left >> right);
            }
            OpType::CalculateMerkleRoot => todo!(),
            OpType::GetUserId => {
                self.targets.push(self.ctx.execution_context.user_id);
            }
            OpType::GetContractId => {
                self.targets.push(self.ctx.execution_context.contract_id);
            }
            OpType::GetCheckpointId => {
                self.targets.push(self.ctx.execution_context.checkpoint_id);
            }
            OpType::GetNonce => {
                self.targets.push(self.ctx.execution_context.nonce);
            }
            OpType::GetUserPublicKeyHash => {
                self.hashes
                    .push(self.ctx.execution_context.user_public_key_hash);
            }
            OpType::GetStateQueryResult => {
                todo!()
            }
            OpType::GetStateQueryResultSingle => {
                assert!(
                    op.inputs.len() == 1,
                    "expected 1 inputs for GetStateQueryResult op, got {} inputs",
                    op.inputs.len()
                );
                let query_index = op.inputs[0] as usize;

                assert!(
                    query_index >= self.ctx.execution_context.state_query_results.len(),
                    "GetStateQueryResult: attempted to read query index {}, but only {} results were provided",
                    query_index,
                    self.ctx.execution_context.state_query_results.len(),
                );

                let result = self.ctx.execution_context.state_query_results[query_index];
                assert!(
                    result.len() == 4,
                    "GetStateQueryResult: attempted to read query index {}, but the length of the result was {}, it should be 4",
                    query_index,
                    result.len(),
                );
                self.hashes.push(result);
            }
            OpType::GetStateCommandResultHash => todo!(),
            OpType::GetStateCommandResultSingle => todo!(),
            OpType::GetStateCommandResultArray => todo!(),
            OpType::UnaryInverse => {
                assert!(
                    op.inputs.len() == 1,
                    "expected 1 input for UnaryInverse op, got {} inputs",
                    op.inputs.len()
                );
                let input = self.resolve_target(op.inputs[0]);
                self.targets.push(input.inverse());
            }
            OpType::UnaryNegative => {
                assert!(
                    op.inputs.len() == 1,
                    "expected 1 input for UnaryNegative op, got {} inputs",
                    op.inputs.len()
                );
                let input = self.resolve_target(op.inputs[0]);
                self.targets.push(-input);
            }
        }
    }
}

pub struct IExecutionContext<F: RichField> {
    pub state_query_results: Vec<[F; 4]>,
    pub user_id: F,
    pub contract_id: F,
    pub checkpoint_id: F,
    pub nonce: F,
    pub user_public_key_hash: [F; 4],
}

impl<F: RichField> IExecutionContext<F> {
    pub fn new() -> Self {
        IExecutionContext {
            state_query_results: Vec::new(),
            user_id: F::ZERO,
            contract_id: F::ZERO,
            checkpoint_id: F::ZERO,
            nonce: F::ZERO,
            user_public_key_hash: [F::ZERO; 4],
        }
    }
}

pub struct IExtendedExecutionContext<F: RichField> {
    pub exec_inputs: Vec<u64>,
    pub def: DPNFunctionCircuitDefinition,
    pub execution_context: IExecutionContext<F>,
    // pub circuit_context: QCircuitContext,
}

impl<F: RichField> IExtendedExecutionContext<F> {
    pub fn new(
        exec_inputs: Vec<u64>,
        def: DPNFunctionCircuitDefinition,
        execution_context: IExecutionContext<F>,
        // circuit_context: QCircuitContext,
    ) -> Self {
        IExtendedExecutionContext {
            exec_inputs,
            def,
            execution_context,
            // circuit_context,
        }
    }
}
