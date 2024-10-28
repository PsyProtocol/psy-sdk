use crate::{circuit_builder::ToFelts, felt::context_felt::ContextFelt};

pub trait Context<F: ContextFelt> {
    fn add_input(&mut self) -> F;

    fn op_const(&mut self, value: u64) -> F;
    fn op_bool(&mut self, value: bool) -> F;

    fn op_add(&mut self, a: F, b: F) -> F;
    fn op_sub(&mut self, a: F, b: F) -> F;
    fn op_mul(&mut self, a: F, b: F) -> F;
    fn op_div(&mut self, a: F, b: F) -> F;
    fn op_mod(&mut self, a: F, b: F) -> F;
    fn op_bool_and(&mut self, a: F, b: F) -> F;
    fn op_bool_or(&mut self, a: F, b: F) -> F;
    fn op_eq(&mut self, a: F, b: F) -> F;
    fn op_neq(&mut self, a: F, b: F) -> F {
        let res = self.op_eq(a, b);
        self.op_bool_not(res)
    }
    fn op_lt(&mut self, a: F, b: F) -> F;
    fn op_lte(&mut self, a: F, b: F) -> F;
    fn op_gt(&mut self, a: F, b: F) -> F;
    fn op_gte(&mut self, a: F, b: F) -> F;
    fn op_bit_shr(&mut self, a: F, b: F) -> F;
    fn op_bit_shl(&mut self, a: F, b: F) -> F;
    fn op_bit_and(&mut self, a: F, b: F) -> F;
    fn op_bit_or(&mut self, a: F, b: F) -> F;
    fn op_bit_xor(&mut self, a: F, b: F) -> F;

    fn op_bool_or_many(&mut self, values: &[F]) -> F {
        let mut result = values[0];
        for i in 1..values.len() {
            result = self.op_bool_or(result, values[i]);
        }
        result
    }
    fn op_bool_and_many(&mut self, values: &[F]) -> F {
        let mut result = values[0];
        for i in 1..values.len() {
            result = self.op_bool_and(result, values[i]);
        }
        result
    }

    fn op_bool_not(&mut self, a: F) -> F;
    fn op_neg(&mut self, a: F) -> F;
    fn op_inverse(&mut self, a: F) -> F;

    fn op_select(&mut self, cond: F, a: F, b: F) -> F;

    fn op_hash(&mut self, values: &[F]) -> [F; 4];

    fn assert_eq(&mut self, a: F, b: F);
    fn assert(&mut self, value: F) {
        let true_value = self.op_bool(true);
        self.assert_eq(value, true_value);
    }

    fn start_if_block(&mut self, a: F);
    fn start_else_if_block(&mut self, a: F);
    fn start_else_block(&mut self);
    fn end_if_block(&mut self);
    fn resolve_current_condition(&mut self) -> F;
    fn cset<V: ToFelts<F>>(&mut self, old_value: V, new_value: V) -> V;

    fn op_target_at(&mut self, parent: F, index: u64) -> F;
    fn op_target_at_vec(&mut self, parent: F, length: u64) -> Vec<F> {
        (0..length).map(|i| self.op_target_at(parent, i)).collect()
    }
    fn op_target_at_array<const N: usize>(&mut self, parent: F) -> [F; N] {
        core::array::from_fn(|i| self.op_target_at(parent, i as u64))
    }
}
