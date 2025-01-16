use crate::{circuit_builder::ToFelts, felt::context_felt::ContextFelt};

pub trait Context<F: ContextFelt> {
    fn get_value(&mut self, a: F) -> u64;
    fn get_bool_value(&mut self, a: F) -> bool;
    fn op_cast_u32(&mut self, a: F) -> F;
    fn op_select(&mut self, condition: F, a: F, b: F) -> F;
    fn op_const(&mut self, value: u64) -> F;
    fn op_bool_not(&mut self, a: F) -> F;
    fn op_bool_and(&mut self, a: F, b: F) -> F;
    fn op_bool_or(&mut self, a: F, b: F) -> F;
    fn op_bool_or_many(&mut self, values: &[F]) -> F;
    fn op_bool_and_many(&mut self, values: &[F]) -> F;
    fn op_add(&mut self, a: F, b: F) -> F;
    fn op_sub(&mut self, a: F, b: F) -> F;
    fn op_mul(&mut self, a: F, b: F) -> F;
    fn op_div(&mut self, a: F, b: F) -> F;
    fn op_mod(&mut self, a: F, b: F) -> F;
    fn op_exp(&mut self, a: F, b: F) -> F;
    fn op_eq(&mut self, a: F, b: F) -> F;
    fn op_neq(&mut self, a: F, b: F) -> F;
    fn op_lt(&mut self, a: F, b: F) -> F;
    fn op_lte(&mut self, a: F, b: F) -> F;
    fn op_gt(&mut self, a: F, b: F) -> F;
    fn op_gte(&mut self, a: F, b: F) -> F;

    // start u32 ops
    fn op_u32_xor(&mut self, a: F, b: F) -> F;
    fn op_u32_or(&mut self, a: F, b: F) -> F;
    fn op_u32_and(&mut self, a: F, b: F) -> F;
    fn op_u32_shl(&mut self, a: F, b: F) -> F;
    fn op_u32_shr(&mut self, a: F, b: F) -> F;

    // end u32 ops
    fn op_bool(&mut self, value: bool) -> F;
    fn op_true(&mut self) -> F;

    fn op_false(&mut self) -> F;

    // unary ops
    fn op_neg(&mut self, a: F) -> F;
    fn op_inverse(&mut self, a: F) -> F;

    fn add_input(&mut self) -> F;
    fn add_inputs(&mut self, count: u64) -> Vec<F>;
    fn assert_eq(&mut self, left: F, right: F, message: &'static str);
    fn assert_true(&mut self, left: F, message: &'static str);
    fn cset<V: ToFelts<F>>(&mut self, old_value: V, new_value: V) -> V;
    fn cset_state<V: ToFelts<F>>(&mut self, old_value: V, new_value: V) -> V;
    fn cset_str<V: ToFelts<F>>(&mut self, left: &'static str, old_value: V, new_value: V) -> V;
    fn start_if_block(&mut self, condition: F);
    fn start_else_if_block(&mut self, condition: F);
    fn start_else_block(&mut self);
    fn end_if_block(&mut self);
    fn resolve_current_condition(&mut self) -> F;
    fn pop_condition(&mut self);

    // std lib
    fn hash(&mut self, values: &[F]) -> [F; 4];
    fn split_bits(&mut self, value: F, num_bits: u64) -> Vec<F>;
    fn sum_bits(&mut self, bits: &[F]) -> F;

    fn get_user_id(&mut self) -> F;
    fn get_contract_id(&mut self) -> F;
    fn get_checkpoint_id(&mut self) -> F;
    fn get_last_nonce(&mut self) -> F;
    fn get_user_public_key_hash(&mut self) -> [F; 4];

    // state operations
    fn op_get_state_felt(
        &mut self,
        contract_state_tree_height: u16,
        contract_id: F,
        user_id: F,
        index: F,
    ) -> F;
    fn op_set_state_felt(&mut self, index: F, value: F) -> F;
    fn op_set_state_obj<T: ToFelts<F>>(&mut self, index: F, value: T) -> T;
    fn cselect<V: ToFelts<F>>(&mut self, condition: F, if_true: V, if_false: V) -> V {
        let if_true = if_true.to_felts();
        let if_false = if_false.to_felts();
        let mut result = vec![];
        for i in 0..if_true.len() {
            result.push(self.op_select(condition, if_true[i], if_false[i]));
        }
        ToFelts::from_felts(&result)
    }
}
