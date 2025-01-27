use std::collections::HashMap;

use plonky2::{field::extension::Extendable, hash::hash_types::{HashOutTarget, RichField}, iop::target::{BoolTarget, Target}, plonk::circuit_builder::CircuitBuilder};
use qed_common_circuit::{builder::comparison::CircuitBuilderComparison, hash::base_types::hash160::Hash160Target, u32::arithmetic_u32::U32Target};
use qedlang_core::dpn::ops::op_types::{decode_indexed_op_id, DPNBuiltInDataType, DPNIndexedVarDef, DPNOpType};

const COMPARISON_BITS: usize = 63;
pub struct SimpleDPNBuilder<F: RichField + Extendable<D>, const D: usize>{
    pub targets: Vec<Target>,
    pub target_arrays: Vec<Vec<Target>>,
    pub hashes: Vec<HashOutTarget>,
    pub hash160s: Vec<Hash160Target>,
    pub bools: Vec<BoolTarget>,
    pub bool_arrays: Vec<Vec<BoolTarget>>,
    pub u32s: Vec<U32Target>,
    pub u32_arrays: Vec<Vec<U32Target>>,
    pub user_id: Target,
    pub contract_id: Target,
    pub checkpoint_id: Target,
    pub nonce: Target,
    pub inputs: Vec<Target>,
    pub constant_targets: HashMap<usize, F>
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleDPNBuilder<F, D> {
    pub fn new_with_contract_ctx(inputs: Vec<Target>, user_id: Target, contract_id: Target, checkpoint_id: Target, nonce: Target) -> Self {
        SimpleDPNBuilder {
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
            constant_targets: HashMap::new(),
            
        }
    }
    pub fn push_external_target(&mut self, target: Target) {
        self.targets.push(target);
    }
    pub fn push_external_target_array(&mut self, target: Vec<Target>) {
        self.target_arrays.push(target);
    }
    pub fn push_external_hash(&mut self, target: HashOutTarget) {
        self.hashes.push(target);
    }
    pub fn resolve_bool(&self, builder: &mut CircuitBuilder<F, D>, id: u64) -> BoolTarget {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::Bool => {
                assert!(index < self.bools.len(), "Invalid bool index");
                self.bools[index]
            }
            DPNBuiltInDataType::Target => {
                assert!(index < self.targets.len(), "Invalid target index");

                let b = BoolTarget::new_unsafe(self.targets[index]);
                builder.assert_bool(b);
                b
            },
            
            DPNBuiltInDataType::U32Target => {
                assert!(index < self.u32s.len(), "Invalid u32 index");


                let b = BoolTarget::new_unsafe(self.u32s[index].0);
                builder.assert_bool(b);
                b
            },
            _ => panic!("Invalid data type for bool"),
        }
    }
    pub fn resolve_hash(&self, id: u64) -> HashOutTarget {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::HashOut => {
                assert!(index < self.hashes.len(), "Invalid hash index");
                self.hashes[index]
            },
            _ => panic!("Invalid data type for hash"),
        }
    }
    pub fn resolve_hash160(&self, id: u64) -> Hash160Target {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::HashOut160 => {
                assert!(index < self.hashes.len(), "Invalid hash160 index");
                self.hash160s[index]
            },
            _ => panic!("Invalid data type for hash160"),
        }
    }
    pub fn resolve_targets_sized<const N: usize>(&self, ids: &[u64; N]) -> [Target; N] {
        core::array::from_fn(|i| {
            self.resolve_target(ids[i])
        })
    }
    pub fn resolve_targets(&self, ids: &[u64]) -> Vec<Target> {
        ids.iter().map(|id| self.resolve_target(*id)).collect::<Vec<Target>>()
    }
    pub fn resolve_target(&self, id: u64) -> Target {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::Bool => {
                assert!(index < self.bools.len(), "Invalid bool index");
                self.bools[index].target
            },
            DPNBuiltInDataType::Target => {
                assert!(index < self.targets.len(), "Invalid target index");
                self.targets[index]
            },
            
            DPNBuiltInDataType::U32Target => {
                assert!(index < self.u32s.len(), "Invalid u32 index");
                self.u32s[index].0
            },
            _ => panic!("Invalid data type for target"),
        }

    }
    pub fn resolve_u32(&self, id: u64) -> U32Target {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            
            DPNBuiltInDataType::U32Target => {
                assert!(index < self.u32s.len(), "Invalid u32 index");

                self.u32s[index]
            },
            DPNBuiltInDataType::Bool => {
                assert!(index < self.bools.len(), "Invalid bool index");
                U32Target(self.bools[index].target)
            },
            DPNBuiltInDataType::Target => {
                assert!(index < self.targets.len(), "Invalid target index");
                // TODO/SECURITY: range check target
                U32Target(self.targets[index])
            },
            _ => panic!("Invalid data type for U32Target"),
        }

    }
    pub fn resolve_target_array(&self, id: u64) -> Vec<Target> {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::BoolArray => {
                assert!(index < self.bool_arrays.len(), "Invalid bool array index");
                self.bool_arrays[index].iter().map(|b|b.target).collect()
            },
            DPNBuiltInDataType::TargetArray => {
                assert!(index < self.target_arrays.len(), "Invalid target array index");
                self.target_arrays[index].clone()
            },
            
            DPNBuiltInDataType::U32TargetArray => {
                assert!(index < self.u32_arrays.len(), "Invalid u32 array index");

                self.u32_arrays[index].iter().map(|b|b.0).collect()
            },
            _ => panic!("Invalid data type for target array"),
        }

    }
    pub fn resolve_target_array_ref(&self, id: u64, index_id: u64) -> Target {
        let (t, index) = decode_indexed_op_id(id);
        let ind_real = self.constant_targets.get(&index).unwrap();
        //let ind_real = self.resolve_target(index_id);
        match t {
            DPNBuiltInDataType::HashOut => {
                assert!(ind_real.to_canonical_u64() < 4, "Invalid index in hash");
                self.hashes[index].elements[ind_real.to_canonical_u64() as usize]
            },
            DPNBuiltInDataType::HashOut160 => {
                assert!(ind_real.to_canonical_u64() < 5, "Invalid index in hash160");
                self.hash160s[index][ind_real.to_canonical_u64() as usize].0
            },
            DPNBuiltInDataType::BoolArray => {
                assert!(index < self.bool_arrays.len(), "Invalid bool array index");
                self.bool_arrays[index][ind_real.to_canonical_u64() as usize].target
            },
            DPNBuiltInDataType::TargetArray => {
                assert!(index < self.target_arrays.len(), "Invalid target array index");
                self.target_arrays[index][ind_real.to_canonical_u64() as usize]
            },
            
            DPNBuiltInDataType::U32TargetArray => {
                assert!(index < self.u32_arrays.len(), "Invalid u32 array index");
                self.u32_arrays[index][ind_real.to_canonical_u64() as usize].0
            },
            _ => panic!("Invalid data type for target array"),
        }

    }
    pub fn resolve_bool_array(&self, id: u64) -> Vec<BoolTarget> {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::BoolArray => {
                assert!(index < self.bool_arrays.len(), "Invalid bool array index");
                self.bool_arrays[index].clone()
            },
            _ => panic!("Invalid data type for bool array"),
        }
    }
    pub fn resolve_u32_array(&self, id: u64) -> Vec<U32Target> {
        let (t, index) = decode_indexed_op_id(id);
        match t {
            DPNBuiltInDataType::U32TargetArray => {
                assert!(index < self.u32_arrays.len(), "Invalid u32 array index");
                self.u32_arrays[index].clone()
            },
            _ => panic!("Invalid data type for bool array"),
        }
    }


    pub fn process_var_def(&mut self, builder: &mut CircuitBuilder<F, D>, op: &DPNIndexedVarDef) {
        
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
            DPNOpType::Constant => {
                self.constant_targets.insert(self.targets.len(), F::from_noncanonical_u64(op.inputs[0]));
                
                self.targets.push(builder.constant(F::from_noncanonical_u64(op.inputs[0])))
            },
            DPNOpType::ConstantTrue => self.bools.push(builder._true()),
            DPNOpType::ConstantFalse => self.bools.push(builder._false()),
            DPNOpType::Add => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(builder.add(left, right));
            },
            DPNOpType::Sub => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(builder.sub(left, right));
            },
            DPNOpType::Mul => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(builder.mul(left, right));
            },
            DPNOpType::Div => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(builder.div(left, right));
            },
            DPNOpType::BoolNot => {
                let left = self.resolve_bool(builder, op.inputs[0]);
                self.bools.push(builder.not(left));
            },
            
            DPNOpType::BoolAnd =>{
                let left = self.resolve_bool(builder,op.inputs[0]);
                let right = self.resolve_bool(builder,op.inputs[1]);
                self.bools.push(builder.and(left, right));
            },
            DPNOpType::BoolOr => {
                let left = self.resolve_bool(builder,op.inputs[0]);
                let right = self.resolve_bool(builder,op.inputs[1]);
                self.bools.push(builder.or(left, right));
            },
            DPNOpType::Xor => todo!(),
            DPNOpType::Nor => todo!(),
            DPNOpType::Eq => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.bools.push(builder.is_equal(left, right));
            },
            DPNOpType::Lte => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.bools.push(builder.is_less_than_or_equal(COMPARISON_BITS, left, right))
            },
            DPNOpType::Gte => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.bools.push(builder.is_greater_than_or_equal(COMPARISON_BITS, left, right))
            },
            DPNOpType::Gt => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.bools.push(builder.is_greater_than(COMPARISON_BITS, left, right))
            },
            DPNOpType::Lt => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.bools.push(builder.is_less_than(COMPARISON_BITS, left, right))
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
                let zero = builder.zero();
                let is_condition_zero = builder.is_equal(condition, zero);
                let x = self.resolve_target(op.inputs[1]);
                let y = self.resolve_target(op.inputs[2]);

                // if condition != 0, then { x } else { y }
                // this is the same as: if condition == 0 then { y } else { x }
                self.targets.push(builder.select(is_condition_zero, y, x));
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