use plonky2::{field::{goldilocks_field::GoldilocksField, types::{Field, Field64, PrimeField64}}, hash::poseidon::PoseidonHash, plonk::config::{GenericHashOut, Hasher}};

use crate::dpn::ops::{context_trait::DPNContext, op_types::DPNOpType, sym_felt::SymFeltRef};

use super::core::RuntimeFelt;


#[derive(Debug, Clone)]
pub struct QRuntimeContext {

}
fn split_bits(x: u64, num_bits: u64) -> Vec<u64> {
    let mut result = vec![0u64; num_bits as usize];
    for i in 0..num_bits {
        result[i as usize] = (x >> i) & 1;
    }
    result
}
fn sum_bits(bits: &[u64]) -> u64 {
    assert!(bits.len() <= 64, "cannot sum more than 64 bits");
    let result = bits.iter().fold(0, |acc, x| acc + x);
    GoldilocksField::from_noncanonical_u64(result).to_canonical_u64()
}
impl QRuntimeContext {
    pub fn new() -> Self {
        QRuntimeContext {
        }
    }

    fn op_std_binary_op(&mut self, op_type: DPNOpType, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt(op_type.eval_binary_constant(a.0, b.0))
    }
    fn op_std_binary_op_u32(&mut self, op_type: DPNOpType, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        
        RuntimeFelt(op_type.eval_binary_constant(a.0&0xFFFFFFFFu64, b.0&0xFFFFFFFFu64)&0xFFFFFFFFu64)
    }
    /*
    fn op_std_unary_op(&mut self, op_type: DPNOpType, a: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt(op_type.eval_unary_constant(a.0))
    }
    fn op_valueless(&mut self, op_type: DPNOpType) -> RuntimeFelt {
        RuntimeFelt(SymFeltRef::new_valueless(op_type).get_constant_value())
    }*/
}


impl DPNContext<RuntimeFelt> for QRuntimeContext {
    fn op_cast_u32(&mut self, a: RuntimeFelt) -> RuntimeFelt {
        a&0xFFFFFFFFu64
    }

    fn op_select(&mut self, condition: RuntimeFelt, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        if condition.0 != 0 {
            a
        } else {
            b
        }
    }

    fn op_const(&mut self, value: u64) -> RuntimeFelt {
        RuntimeFelt(value%GoldilocksField::ORDER)
    }

    fn op_bool_not(&mut self, a: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt((a.0 == 0) as u64)
    }

    fn op_bool_and(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt((a.0 != 0 && b.0 != 0) as u64)
    }

    fn op_bool_or(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt((a.0 != 0 || b.0 != 0) as u64)
    }

    fn op_bool_or_many(&mut self, values: &[RuntimeFelt]) -> RuntimeFelt {
        let mut result = values[0];
        for i in 1..values.len() {
            result = self.op_bool_or(result, values[i]);
        }
        result
    }

    fn op_bool_and_many(&mut self, values: &[RuntimeFelt]) -> RuntimeFelt {        let mut result = values[0];
        for i in 1..values.len() {
            result = self.op_bool_and(result, values[i]);
        }
        result
    }

    fn op_add(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op(DPNOpType::Add, a, b)
    }
    fn op_sub(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op(DPNOpType::Sub, a, b)
    }
    fn op_mul(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op(DPNOpType::Mul, a, b)
    }
    fn op_div(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op(DPNOpType::Div, a, b)
    }
    fn op_mod(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op(DPNOpType::Mod, a, b)
    }
    fn op_exp(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op(DPNOpType::Exp, a, b)
    }
    fn op_eq(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op(DPNOpType::Eq, a, b)
    }
    fn op_neq(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        let eq = self.op_std_binary_op(DPNOpType::Eq, a, b);
        self.op_bool_not(eq)
    }
    fn op_lt(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op(DPNOpType::Lt, a, b)
    }
    fn op_lte(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        let lt = self.op_std_binary_op(DPNOpType::Lt, a, b);
        let eq = self.op_std_binary_op(DPNOpType::Eq, a, b);
        self.op_bool_or(lt, eq)
    }
    fn op_gt(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        let lt = self.op_std_binary_op(DPNOpType::Lt, b, a);
        self.op_bool_not(lt)
    }
    fn op_gte(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        let lt = self.op_std_binary_op(DPNOpType::Lt, b, a);
        let eq = self.op_std_binary_op(DPNOpType::Eq, a, b);
        self.op_bool_or(lt, eq)
    }

    // start u32 ops
    fn op_u32_xor(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op_u32(DPNOpType::U32Xor, a, b)
    }
    fn op_u32_or(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op_u32(DPNOpType::U32Or, a, b)
    }
    fn op_u32_and(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op_u32(DPNOpType::U32And, a, b)
    }
    fn op_u32_shl(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op_u32(DPNOpType::U32ShiftLeft, a, b)
    }
    fn op_u32_shr(&mut self, a: RuntimeFelt, b: RuntimeFelt) -> RuntimeFelt {
        self.op_std_binary_op_u32(DPNOpType::U32ShiftRight, a, b)
    }

    fn op_true(&mut self) -> RuntimeFelt {
       RuntimeFelt(1)
    }

    fn op_false(&mut self) -> RuntimeFelt {
        RuntimeFelt(0)
    }

    fn add_input(&mut self) -> RuntimeFelt {
        panic!("add_input not implemented in QRuntimeContext")
    }

    fn add_inputs(&mut self, count: u64) -> Vec<RuntimeFelt> {
        panic!("add_inputs not implemented in QRuntimeContext")
    }

    fn assert_eq(&mut self, left: RuntimeFelt, right: RuntimeFelt, message: &'static str) {
        assert!(left.0 == right.0, "{}", message);
    }

    fn assert_true(&mut self, left: RuntimeFelt, message: &'static str) {
        assert!(left.0 != 0, "{}", message);
    }

    fn cset(&mut self, _old_value: RuntimeFelt, new_value: RuntimeFelt) -> RuntimeFelt {
        new_value
    }

    fn start_if_block(&mut self, _condition: RuntimeFelt) {
        // no-op
    }

    fn start_else_if_block(&mut self, _condition: RuntimeFelt) {
        // no-op
    }

    fn start_else_block(&mut self) {
        // no-op
    }

    fn end_if_block(&mut self) {
        // no-op
    }

    fn resolve_current_condition(&mut self) -> RuntimeFelt {
        RuntimeFelt(1)
    }

    fn pop_condition(&mut self) {
        // no-op
    }

    fn hash(&mut self, values: &[RuntimeFelt]) -> [RuntimeFelt; 4] {
        let gl_values = values.iter().map(|v| GoldilocksField::from_noncanonical_u64(v.0)).collect::<Vec<GoldilocksField>>();
        let res = PoseidonHash::hash_no_pad(&gl_values).to_vec();
        [
            RuntimeFelt(res[0].to_canonical_u64()),
            RuntimeFelt(res[1].to_canonical_u64()),
            RuntimeFelt(res[2].to_canonical_u64()),
            RuntimeFelt(res[3].to_canonical_u64()),
        ]
    }

    fn split_bits(&mut self, value: RuntimeFelt, num_bits: u64) -> Vec<RuntimeFelt> {
        split_bits(value.0, num_bits).iter().map(|x| RuntimeFelt(*x)).collect()
    }
    fn sum_bits(&mut self, bits: &[RuntimeFelt]) -> RuntimeFelt {
        let bits_u64 = bits.iter().map(|x| x.0).collect::<Vec<u64>>();
        RuntimeFelt(sum_bits(&bits_u64))
    }
}