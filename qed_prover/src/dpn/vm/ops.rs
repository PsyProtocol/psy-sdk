use std::collections::HashMap;

use plonky2::{field::extension::Extendable, hash::hash_types::{HashOutTarget, RichField}, iop::target::{BoolTarget, Target}, plonk::circuit_builder::CircuitBuilder};
use qed_common_circuit::{builder::comparison::CircuitBuilderComparison, crypto::secp256k1::ecdsa::gadgets::biguint::{BigUintTarget, CircuitBuilderBiguint}, hash::base_types::hash160::Hash160Target, u32::{arithmetic_u32::{CircuitBuilderU32, U32Target}, interleaved_u32::CircuitBuilderB32}};
use qed_data::config::store_config::QEDHasher;
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
    pub user_public_key: HashOutTarget,
    pub nonce: Target,
    pub inputs: Vec<Target>,
    pub constant_targets: HashMap<usize, F>
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleDPNBuilder<F, D> {
    pub fn new_with_contract_ctx(inputs: Vec<Target>, user_id: Target, contract_id: Target, checkpoint_id: Target, nonce: Target, user_public_key: HashOutTarget) -> Self {
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
            user_public_key,
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
        let (_t1, index1) = decode_indexed_op_id(index_id);
        let ind_real = self.constant_targets.get(&index1).unwrap();
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
            DPNOpType::Xor => {
                let left = self.resolve_bool(builder,op.inputs[0]);
                let not_left = builder.not(left);
                let right = self.resolve_bool(builder,op.inputs[1]);
                let not_right = builder.not(right);
                let left_and_not_right = builder.and(left, not_right);
                let not_left_and_right = builder.and(not_left, right);
                self.bools.push(builder.or(left_and_not_right, not_left_and_right));
            },
            DPNOpType::Nor => {
                let left = self.resolve_bool(builder,op.inputs[0]);
                let right = self.resolve_bool(builder,op.inputs[1]);
                let left_or_right = builder.or(left, right);
                self.bools.push(builder.not(left_or_right));
            },
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
            DPNOpType::SplitBits => {
                let target = self.resolve_target(op.inputs[0]);
                self.bool_arrays.push(builder.split_le(target, 64))
            },
            DPNOpType::SumBits => {
                let mut sum = builder.zero();
                op.inputs.iter().for_each(|input|{
                    let bit = self.resolve_bool(builder, *input);
                    sum = builder.add(sum, bit.target);
                });
                self.targets.push(sum);
            },
            DPNOpType::TargetAt => {
                let r = self.resolve_target_array_ref(op.inputs[0], op.inputs[1]);
                self.targets.push(r);
            },
            DPNOpType::HashNoPad => {
                let targets = self.resolve_targets(&op.inputs);
                let output = builder.hash_n_to_hash_no_pad::<QEDHasher>(targets);
                self.hashes.push(output);
            },
            DPNOpType::HashPad => unimplemented!(),
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
            DPNOpType::Exp => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                self.targets.push(builder.exp(left, right, 64))
            },
            DPNOpType::ExpConstantPower => {
                let left = self.resolve_target(op.inputs[0]);
                let (_optype, right) = decode_indexed_op_id(op.inputs[1]);
                self.targets.push(builder.exp_u64(left, right as u64))
            },
            DPNOpType::ExpConstantBase => {
                let (_optype, left) = decode_indexed_op_id(op.inputs[0]);
                let right = builder.split_le(self.resolve_target(op.inputs[1]), 64);
                self.targets.push(builder.exp_from_bits_const_base(F::from_noncanonical_u64(left as u64), right))
            }
            DPNOpType::Mod => {
                let left = self.resolve_target(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                let (left_low, left_high) = qed_common_circuit::builder::core::CircuitBuilderHelpersCore::split_low_high_32bits( builder, left);
                let (right_low, right_high) = qed_common_circuit::builder::core::CircuitBuilderHelpersCore::split_low_high_32bits( builder, right);
                let left_biguint = BigUintTarget{
                    limbs: vec![U32Target(left_high), U32Target(left_low)],
                };
                let right_biguint = BigUintTarget{
                    limbs: vec![U32Target(right_high), U32Target(right_low)],
                };
                let (_div_biguint, rem_biguint) = builder.div_rem_biguint(&left_biguint, &right_biguint);
                assert!(rem_biguint.limbs.len() == 2, "Felt Mod should return two limb");
                let twopow32 = qed_common_circuit::builder::core::CircuitBuilderHelpersCore::constant_u64(builder, 0x100000000);
                let res = builder.mul_add(rem_biguint.limbs[0].0, twopow32, rem_biguint.limbs[1].0);
                self.targets.push(res);
            }
            DPNOpType::ModConstantDividend => {
               let (_optype, left) = decode_indexed_op_id(op.inputs[0]);
                let right = self.resolve_target(op.inputs[1]);
                
                let (left_low, left_high) = (builder.constant(F::from_noncanonical_u64((left as u64) & 0xffffffffu64)), builder.constant(F::from_noncanonical_u64((left as u64) >> 32u64)));
                let (right_low, right_high) = qed_common_circuit::builder::core::CircuitBuilderHelpersCore::split_low_high_32bits( builder, right);
                let left_biguint = BigUintTarget{
                    limbs: vec![U32Target(left_high), U32Target(left_low)],
                };
                let right_biguint = BigUintTarget{
                    limbs: vec![U32Target(right_high), U32Target(right_low)],
                };
                let (_div_biguint, rem_biguint) = builder.div_rem_biguint(&left_biguint, &right_biguint);
                assert!(rem_biguint.limbs.len() == 2, "Felt Mod should return two limb");
                let twopow32 = qed_common_circuit::builder::core::CircuitBuilderHelpersCore::constant_u64(builder, 0x100000000);
                let res = builder.mul_add(rem_biguint.limbs[0].0, twopow32, rem_biguint.limbs[1].0);
                self.targets.push(res);
            }
            DPNOpType::ModConstantDivisor => {
                let left = self.resolve_target(op.inputs[0]);
                let (_optype, right) = decode_indexed_op_id(op.inputs[1]);
                let (left_low, left_high) = qed_common_circuit::builder::core::CircuitBuilderHelpersCore::split_low_high_32bits( builder, left);
                let (right_low, right_high) = (builder.constant(F::from_noncanonical_u64((right as u64) & 0xffffffffu64)), builder.constant(F::from_noncanonical_u64((right as u64) >> 32u64)));
                let left_biguint = BigUintTarget{
                    limbs: vec![U32Target(left_high), U32Target(left_low)],
                };
                let right_biguint = BigUintTarget{
                    limbs: vec![U32Target(right_high), U32Target(right_low)],
                };
                let (_div_biguint, rem_biguint) = builder.div_rem_biguint(&left_biguint, &right_biguint);
                assert!(rem_biguint.limbs.len() == 2, "Felt Mod should return two limb");
                let twopow32 = qed_common_circuit::builder::core::CircuitBuilderHelpersCore::constant_u64(builder, 0x100000000);
                let res = builder.mul_add(rem_biguint.limbs[0].0, twopow32, rem_biguint.limbs[1].0);
                self.targets.push(res);
            },
            DPNOpType::DivRem4 => {
                let target = self.resolve_target(op.inputs[0]);
                let (low, high) = builder.split_low_high(target, 2, 64);
                self.target_arrays.push(vec![high, low]);
            },
            DPNOpType::CastU32 => {
                let target = self.resolve_target(op.inputs[0]);
                let (low, high) = qed_common_circuit::builder::core::CircuitBuilderHelpersCore::split_low_high_32bits( builder, target);
                builder.assert_zero(high);
                self.u32s.push(U32Target(low));
            }
            DPNOpType::U32And => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(builder.and_u32(left, right));
            },
            DPNOpType::U32AndConstant => {
                let left = self.resolve_u32(op.inputs[0]);
                let (_op_type, right) = decode_indexed_op_id(op.inputs[1]);
                let right = builder.constant_u32(right as u32);
                self.u32s.push(builder.and_u32(left, right));
            },
            DPNOpType::U32Or => {
                let neg_left = builder.not_u32(self.resolve_u32(op.inputs[0]));
                let neg_right = builder.not_u32(self.resolve_u32(op.inputs[1]));
                let neg_left_or_right = builder.and_u32(neg_left, neg_right);
                self.u32s.push(builder.not_u32(neg_left_or_right));
            },
            DPNOpType::U32OrConstant => {
                let neg_left = builder.not_u32(self.resolve_u32(op.inputs[0]));
                let (_op_type, right) = decode_indexed_op_id(op.inputs[1]);
                let neg_right = builder.constant_u32(0xffffffff - (right as u32));
                let neg_left_or_right = builder.and_u32(neg_left, neg_right);
                self.u32s.push(builder.not_u32(neg_left_or_right));
            },
            DPNOpType::U32Xor => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                self.u32s.push(builder.xor_u32(left, right));
            },
            DPNOpType::U32XorConstant => {
                let left = self.resolve_u32(op.inputs[0]);
                let (_op_type, right) = decode_indexed_op_id(op.inputs[1]);
                let right = builder.constant_u32(right as u32);
                self.u32s.push(builder.xor_u32(left, right));
            },
            DPNOpType::U32ShiftLeft => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                let two = builder.two();
                let power_of_two = U32Target(builder.exp(two, right.0, 32));
                self.u32s.push(builder.mul_u32(left, power_of_two).0);
            },
            DPNOpType::U32ShiftLeftConstantBitDistance => {
                let left = self.resolve_u32(op.inputs[0]);
                let (_op_type, right) = decode_indexed_op_id(op.inputs[1]);
                self.u32s.push(builder.lsh_u32(left, right as u8));
            },
            DPNOpType::U32ShiftLeftConstantValue => {
                let (_op_type, left) = decode_indexed_op_id(op.inputs[1]);
                let left = builder.constant_u32(left as u32);
                let right = self.resolve_u32(op.inputs[1]);
                let two = builder.two();
                let power_of_two = U32Target(builder.exp(two, right.0, 32));
                self.u32s.push(builder.mul_u32(left, power_of_two).0);
            },
            DPNOpType::U32ShiftRight => {
                let (_op_type, left) = decode_indexed_op_id(op.inputs[0]);
                let left = builder.constant_u32(left as u32);
                let u32_bits_len = builder.constant_u32(32);
                let u32_zero = builder.constant_u32(0);
                let (right_exp, right_borrow) = builder.sub_u32(u32_bits_len, self.resolve_u32(op.inputs[1]), u32_zero);
                builder.assert_zero(right_borrow.0);
                let two = builder.two();
                let power_of_two = U32Target(builder.exp(two, right_exp.0, 32));
                self.u32s.push(builder.mul_u32(left, power_of_two).1);
            },
            DPNOpType::U32ShiftRightConstantBitDistance => {
                let left = self.resolve_u32(op.inputs[0]);
                let (_op_type, right) = decode_indexed_op_id(op.inputs[1]);
                self.u32s.push(builder.rsh_u32(left, right as u8));
            },
            DPNOpType::U32ShiftRightConstantValue => {
                let (_op_type, left) = decode_indexed_op_id(op.inputs[0]);
                let left = builder.constant_u32(left as u32);
                let u32_bits_len = builder.constant_u32(32);
                let u32_zero = builder.constant_u32(0);
                let (right_exp, right_borrow) = builder.sub_u32(u32_bits_len, self.resolve_u32(op.inputs[1]), u32_zero);
                builder.assert_zero(right_borrow.0);
                let two = builder.two();
                let power_of_two = U32Target(builder.exp(two, right_exp.0, 32));
                self.u32s.push(builder.mul_u32(left, power_of_two).1);
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
                let target = self.resolve_target(op.inputs[0]);
                builder.assert_non_zero(target);
                self.targets.push(builder.inverse(target));
            },
            DPNOpType::UnaryNegative => {
                let target = self.resolve_target(op.inputs[0]);
                self.targets.push(builder.neg(target));
            },
            DPNOpType::U32InputTarget => {
                let index = op.inputs[0] as usize;
                if index >= self.inputs.len() {
                    panic!("Invalid input index");
                } else {
                    let (low, high) = qed_common_circuit::builder::core::CircuitBuilderHelpersCore::split_low_high_32bits( builder, self.inputs[index]);
                    builder.assert_zero(high);
                    self.u32s.push(U32Target(low));
                }
            },
            DPNOpType::ConstantU32 => {
                assert!(op.inputs[0] <= 0xffffffffu64, "Invalid constant u32");
                let target = builder.constant_u32(op.inputs[0] as u32);
                self.u32s.push(target);
            }
            DPNOpType::U32Add => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                let (low, high) = builder.add_u32(left, right);
                builder.assert_zero(high.0);
                self.u32s.push(low);
            }
            DPNOpType::U32Sub => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                let zero = builder.zero_u32();
                let (low, high) = builder.sub_u32(left, right, zero);
                builder.assert_zero(high.0);
                self.u32s.push(low);
            }
            DPNOpType::U32Mul => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                let (low, high) = builder.mul_u32(left, right);
                builder.assert_zero(high.0);
                self.u32s.push(low);
            }
            DPNOpType::U32Div => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);

                let left_biguint = BigUintTarget{
                    limbs: vec![left],
                };
                let right_biguint = BigUintTarget{
                    limbs: vec![right],
                };
                let div_biguint = builder.div_biguint(&left_biguint, &right_biguint);

                assert!(div_biguint.limbs.len() == 1, "U32Div should only return one limb");

                let div = div_biguint.limbs[0];
                self.u32s.push(div);
            }
            DPNOpType::CastFelt => {
                let target = self.resolve_target(op.inputs[0]);
                self.targets.push(target);
            }
            DPNOpType::CastBool => {
                let target = self.resolve_target(op.inputs[0]);
                let bool_target = BoolTarget::new_unsafe(target);
                builder.assert_bool(bool_target);
                self.bools.push(bool_target);
            }
            DPNOpType::BoolInputTarget => {
                let index = op.inputs[0] as usize;
                if index >= self.inputs.len() {
                    panic!("Invalid input index");
                }
                let bool_target = BoolTarget::new_unsafe(self.inputs[index]);
                builder.assert_bool(bool_target);
                self.bools.push(bool_target);
            }
            DPNOpType::U32Mod => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);

                let left_biguint = BigUintTarget{
                    limbs: vec![left],
                };
                let right_biguint = BigUintTarget{
                    limbs: vec![right],
                };
                let (_div_biguint, rem_biguint) = builder.div_rem_biguint(&left_biguint, &right_biguint);

                assert!(rem_biguint.limbs.len() == 1, "U32 Mod should only return one limb");

                let div = rem_biguint.limbs[0];
                self.u32s.push(div);
            }
            DPNOpType::U32Exp => {
                let left = self.resolve_u32(op.inputs[0]);
                let right = self.resolve_u32(op.inputs[1]);
                let res = builder.exp(left.0, right.0, 32);
                let (low, high) = qed_common_circuit::builder::core::CircuitBuilderHelpersCore::split_low_high_32bits( builder, res);
                builder.assert_zero(high);
                self.u32s.push(U32Target(low));
            }
        }
        
    }
}