use plonky2::{field::{goldilocks_field::GoldilocksField, types::{Field, Field64, PrimeField64}}, hash::poseidon::PoseidonHash, plonk::config::{GenericHashOut, Hasher}};

use crate::dpn::ops::{context_trait::{ContextFelt, DPNContext, ToFelts}, op_types::DPNOpType, sym_felt::SymFeltRef};



#[derive(Debug, Clone)]
pub struct QRuntimeContext<F: ContextFelt> {
    _phantom: std::marker::PhantomData<F>,

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
impl<F: ContextFelt> QRuntimeContext<F> {
    pub fn new() -> Self {
        QRuntimeContext {
            _phantom: std::marker::PhantomData,
        }
    }

    fn op_std_binary_op(&mut self, op_type: DPNOpType, a: F, b: F) -> F {
        F::cns(op_type.eval_binary_constant(a.get_u64(), b.get_u64()))
    }
    fn op_std_binary_op_u32(&mut self, op_type: DPNOpType, a: F, b: F) -> F {

        F::cns(op_type.eval_binary_constant(a.get_u64()&0xFFFFFFFFu64, b.get_u64()&0xFFFFFFFFu64)&0xFFFFFFFFu64)
    }
    /*
    fn op_std_unary_op(&mut self, op_type: DPNOpType, a: F) -> F {
        F::cns(op_type.eval_unary_constant(a.get_u64()))
    }
    fn op_valueless(&mut self, op_type: DPNOpType) -> F {
        F::cns(SymFeltRef::new_valueless(op_type).get_constant_value())
    }*/
}


impl<F: ContextFelt> DPNContext<F> for QRuntimeContext<F> {
    fn get_constant_value(&self, a: F) -> u64 {
        a.get_u64()
    }
    fn get_op_type(&self, a: F) -> DPNOpType {
        unimplemented!()
    }

    fn op_cast_u32(&mut self, a: F) -> F {
        a&0xFFFFFFFFu64
    }

    fn op_cast_felt(&mut self, a: F) -> F {
        a&0xFFFFFFFFu64
    }

    fn op_cast_bool(&mut self, a: F) -> F {
        unimplemented!()
    }

    fn op_select(&mut self, condition: F, a: F, b: F) -> F {
        if condition.get_u64() != 0 {
            a
        } else {
            b
        }
    }

    fn op_const(&mut self, value: u64) -> F {
        F::cns(value%GoldilocksField::ORDER)
    }

    fn op_const_u32(&mut self, value: u32) -> F {
        F::cns(value as u64)
    }

    fn op_bool_not(&mut self, a: F) -> F {
        F::cns((a.get_u64() == 0) as u64)
    }

    fn op_bool_and(&mut self, a: F, b: F) -> F {
        F::cns((a.get_u64() != 0 && b.get_u64() != 0) as u64)
    }

    fn op_bool_or(&mut self, a: F, b: F) -> F {
        F::cns((a.get_u64() != 0 || b.get_u64() != 0) as u64)
    }

    fn op_bool_or_many(&mut self, values: &[F]) -> F {
        let mut result = values[0];
        for i in 1..values.len() {
            result = self.op_bool_or(result, values[i]);
        }
        result
    }

    fn op_bool_and_many(&mut self, values: &[F]) -> F {        let mut result = values[0];
        for i in 1..values.len() {
            result = self.op_bool_and(result, values[i]);
        }
        result
    }

    fn op_add(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Add, a, b)
    }
    fn op_sub(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Sub, a, b)
    }
    fn op_mul(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Mul, a, b)
    }
    fn op_div(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Div, a, b)
    }
    fn op_u32_add(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Add, a, b)
    }
    fn op_u32_sub(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Sub, a, b)
    }
    fn op_u32_mul(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Mul, a, b)
    }
    fn op_u32_div(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Div, a, b)
    }
    fn op_mod(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Mod, a, b)
    }
    fn op_exp(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Exp, a, b)
    }
    fn op_u32_mod(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op_u32(DPNOpType::U32Mod, a, b)
    }
    fn op_u32_exp(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op_u32(DPNOpType::U32Exp, a, b)
    }
    fn op_eq(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Eq, a, b)
    }
    fn op_neq(&mut self, a: F, b: F) -> F {
        let eq = self.op_std_binary_op(DPNOpType::Eq, a, b);
        self.op_bool_not(eq)
    }
    fn op_lt(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op(DPNOpType::Lt, a, b)
    }
    fn op_lte(&mut self, a: F, b: F) -> F {
        let lt = self.op_std_binary_op(DPNOpType::Lt, a, b);
        let eq = self.op_std_binary_op(DPNOpType::Eq, a, b);
        self.op_bool_or(lt, eq)
    }
    fn op_gt(&mut self, a: F, b: F) -> F {
        let lt = self.op_std_binary_op(DPNOpType::Lt, b, a);
        self.op_bool_not(lt)
    }
    fn op_gte(&mut self, a: F, b: F) -> F {
        let lt = self.op_std_binary_op(DPNOpType::Lt, b, a);
        let eq = self.op_std_binary_op(DPNOpType::Eq, a, b);
        self.op_bool_or(lt, eq)
    }

    // start u32 ops
    fn op_u32_xor(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op_u32(DPNOpType::U32Xor, a, b)
    }
    fn op_u32_or(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op_u32(DPNOpType::U32Or, a, b)
    }
    fn op_u32_and(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op_u32(DPNOpType::U32And, a, b)
    }
    fn op_u32_shl(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op_u32(DPNOpType::U32ShiftLeft, a, b)
    }
    fn op_u32_shr(&mut self, a: F, b: F) -> F {
        self.op_std_binary_op_u32(DPNOpType::U32ShiftRight, a, b)
    }

    fn op_true(&mut self) -> F {
       F::cns(1)
    }

    fn op_false(&mut self) -> F {
        F::cns(0)
    }

    fn add_input(&mut self) -> F {
        panic!("add_input not implemented in QRuntimeContext")
    }

    fn add_u32_input(&mut self) -> F {
        panic!("add_u32_input not implemented in QRuntimeContext")
    }

    fn add_bool_input(&mut self) -> F {
        panic!("add_bool_input not implemented in QRuntimeContext")
    }

    fn add_inputs(&mut self, _count: u64) -> Vec<F> {
        panic!("add_inputs not implemented in QRuntimeContext")
    }

    fn assert_eq(&mut self, left: F, right: F, message: &'static str) {
        assert!(left.get_u64() == right.get_u64(), "{}", message);
    }

    fn assert_true(&mut self, left: F, message: &'static str) {
        assert!(left.get_u64() != 0, "{}", message);
    }

    fn start_if_block(&mut self, _condition: F) {
        // no-op
    }

    fn start_else_if_block(&mut self, _condition: F) {
        // no-op
    }

    fn start_else_block(&mut self) {
        // no-op
    }

    fn end_if_block(&mut self) {
        // no-op
    }

    fn resolve_current_condition(&mut self) -> F {
        F::cns(1)
    }

    fn pop_condition(&mut self) {
        // no-op
    }

    fn hash(&mut self, values: &[F]) -> [F; 4] {
        let gl_values = values.iter().map(|v| GoldilocksField::from_noncanonical_u64(v.get_u64())).collect::<Vec<GoldilocksField>>();
        let res = PoseidonHash::hash_no_pad(&gl_values).to_vec();
        [
            F::cns(res[0].to_canonical_u64()),
            F::cns(res[1].to_canonical_u64()),
            F::cns(res[2].to_canonical_u64()),
            F::cns(res[3].to_canonical_u64()),
        ]
    }

    fn hash_two_to_one(&mut self, left: &[F; 4], right: &[F; 4]) -> [F; 4] {
        let left_hash = plonky2::hash::hash_types::HashOut {
            elements: [
                GoldilocksField::from_noncanonical_u64(left[0].get_u64()),
                GoldilocksField::from_noncanonical_u64(left[1].get_u64()),
                GoldilocksField::from_noncanonical_u64(left[2].get_u64()),
                GoldilocksField::from_noncanonical_u64(left[3].get_u64()),
            ],
        };
        let right_hash = plonky2::hash::hash_types::HashOut {
            elements: [
                GoldilocksField::from_noncanonical_u64(right[0].get_u64()),
                GoldilocksField::from_noncanonical_u64(right[1].get_u64()),
                GoldilocksField::from_noncanonical_u64(right[2].get_u64()),
                GoldilocksField::from_noncanonical_u64(right[3].get_u64()),
            ],
        };
        let res = PoseidonHash::two_to_one(left_hash, right_hash);
        [
            F::cns(res.elements[0].to_canonical_u64()),
            F::cns(res.elements[1].to_canonical_u64()),
            F::cns(res.elements[2].to_canonical_u64()),
            F::cns(res.elements[3].to_canonical_u64()),
        ]
    }

    fn split_bits(&mut self, value: F, num_bits: u64) -> Vec<F> {
        split_bits(value.get_u64(), num_bits).iter().map(|x| F::cns(*x)).collect()
    }
    fn sum_bits(&mut self, bits: &[F]) -> F {
        let bits_u64 = bits.iter().map(|x| x.get_u64()).collect::<Vec<u64>>();
        F::cns(sum_bits(&bits_u64))
    }

    fn get_user_id(&mut self) -> F {
        todo!()
    }

    fn get_contract_id(&mut self) -> F {
        todo!()
    }

    fn get_checkpoint_id(&mut self) -> F {
        todo!()
    }

    fn get_last_nonce(&mut self) -> F {
        todo!()
    }

    fn get_user_public_key_hash(&mut self) -> [F; 4] {
        todo!()
    }

    fn get_checkpoint_stats(&mut self, _checkpoint_id: F) -> Vec<F> {
        vec![F::cns(0); 36]
    }

    fn get_register_users_root(&mut self, checkpoint_id: F) -> [F; 4] {
        let stats = self.get_checkpoint_stats(checkpoint_id);
        [stats[10], stats[11], stats[12], stats[13]]
    }

    fn get_gutas_root(&mut self, checkpoint_id: F) -> [F; 4] {
        let stats = self.get_checkpoint_stats(checkpoint_id);
        [stats[14], stats[15], stats[16], stats[17]]
    }

    fn get_deploy_contracts_root(&mut self, checkpoint_id: F) -> [F; 4] {
        let stats = self.get_checkpoint_stats(checkpoint_id);
        [stats[18], stats[19], stats[20], stats[21]]
    }

    fn get_fees_collected(&mut self, checkpoint_id: F) -> F {
        let stats = self.get_checkpoint_stats(checkpoint_id);
        stats[0]
    }

    fn get_user_ops_processed(&mut self, checkpoint_id: F) -> F {
        let stats = self.get_checkpoint_stats(checkpoint_id);
        stats[1]
    }

    fn get_total_transactions(&mut self, checkpoint_id: F) -> F {
        let stats = self.get_checkpoint_stats(checkpoint_id);
        stats[2]
    }

    fn get_slots_modified(&mut self, checkpoint_id: F) -> F {
        let stats = self.get_checkpoint_stats(checkpoint_id);
        stats[3]
    }

    fn get_register_users_completed(&mut self, checkpoint_id: F) -> F {
        let stats = self.get_checkpoint_stats(checkpoint_id);
        stats[5] // pm_jobs_completed.register_users_completed
    }

    fn get_gutas_completed(&mut self, checkpoint_id: F) -> F {
        let stats = self.get_checkpoint_stats(checkpoint_id);
        stats[6] // pm_jobs_completed.gutas_completed
    }

    fn get_deploy_contracts_completed(&mut self, checkpoint_id: F) -> F {
        let stats = self.get_checkpoint_stats(checkpoint_id);
        stats[4] // pm_jobs_completed.deploy_contracts_completed
    }

    fn cset<V: ToFelts<F>>(&mut self, _old_value: V, new_value: V) -> V {
        new_value
    }

    fn cset_str<V: ToFelts<F>>(&mut self, left: &'static str,  old_value: V, new_value: V) -> V {
        println!("cset_str: {}", left);
        self.cset(old_value, new_value)
    }

    fn op_get_state_felt(&mut self, _contract_state_tree_height: u16, _contract_id: F, _user_id: F, _index: F) -> F {
        todo!()
    }

    fn op_set_state_felt(&mut self, _index: F, _value: F) -> F {
        todo!()
    }

    fn op_set_state_obj<T: ToFelts<F>>(&mut self, _index: F, _value: T) -> T {
        todo!()
    }

    fn clear_entire_tree(&mut self) -> Vec<F> {
        todo!()
    }

    fn cset_state<V: ToFelts<F>>(&mut self, _old_value: V, _new_value: V) -> V {
        todo!()
    }

    fn cset_state_at<V: ToFelts<F>>(&mut self, _sub_index: F, _new_value: V) -> V {
        todo!()
    }

    fn cset_state_hash_at(&mut self, _slot_index: F, _new_value: [F; 4]) -> [F; 4] {
        todo!()
    }

    fn get_state_hash_at(&mut self, _slot_index: F) -> [F; 4] {
        todo!()
    }

    fn cinvoke_external_contract_function_sync(
        &mut self,
        _contract_id: F,
        _method_id: F,
        _inputs: Vec<F>,
        num_outputs: u32,
    ) -> Vec<F> {
        todo!()
    }

    fn cinvoke_external_contract_function_deferred(
        &mut self,
        _contract_id: F,
        _method_id: F,
        _inputs: Vec<F>,
    ) -> [F; 4] {
        todo!()
    }

    fn get_other_contract_state_hash_at(&mut self, contract_state_tree_height: F, contract_id: F, slot_index: F) -> [F; 4] {
        todo!()
    }

    fn get_other_user_contract_state_hash_at(&mut self, contract_state_tree_height: F, user_id: F, contract_id: F, slot_index: F) -> [F; 4] {
        todo!()
    }

    fn get_state_range_at(&mut self, sub_slot_index: F, length: F) -> Vec<F> {
        todo!()
    }

    fn cset_state_range_at(&mut self, sub_slot_index: F, values: &[F]) {
        todo!()
    }

    fn get_other_user_contract_state_range_at(&mut self, contract_state_tree_height: F, user_id: F, contract_id: F, sub_slot_index: F, length: F) -> Vec<F> {
        todo!()
    }

    fn op_check_secp_sign(&mut self, public_key: [F; 16], msg_hash: [F; 4], signature: [F; 16]) -> F {
        todo!()
    }
}
