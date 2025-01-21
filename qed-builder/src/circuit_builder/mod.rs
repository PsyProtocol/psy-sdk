use crate::{
    felt::sym_felt::{SetSymFeltRef, SymFeltRefValue, SymFeltStore, SymRefAssertion},
    ops::{DPNBuiltInDataType, OpType},
    Context, ContextFelt, SymFeltRef,
};

pub mod plonky2;

#[derive(Debug, Clone)]
pub struct IfConditionStack {
    pub conditions: Vec<SymFeltRef>,
    pub current_condition: SymFeltRef,
}
#[derive(Debug, Clone)]
pub struct ExecContext {
    pub store: SymFeltStore,
    pub input_count: u64,
    pub assertions: Vec<SymRefAssertion>,
    pub set_state_commands: Vec<SetSymFeltRef>,
    condition_stack: Vec<IfConditionStack>,
    current_condition: SymFeltRef,
    external_function_call_count: u16,
    contract_state_tree_height: u16,
    pub set_state_command_count: u32,
}

impl ExecContext {
    pub fn new() -> Self {
        ExecContext {
            store: SymFeltStore::new(),
            input_count: 0,
            assertions: vec![],
            set_state_commands: vec![],
            condition_stack: vec![],
            current_condition: SymFeltRef::new_valueless(OpType::ConstantTrue),
            external_function_call_count: 0,
            contract_state_tree_height: 32,
            set_state_command_count: 0,
        }
    }

    fn simplify_add(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        let a_type = a.get_op_type();
        let b_type = b.get_op_type();
        if a_type == OpType::Constant && b_type == OpType::Add {
            let b_inner = self.store.get_direct_children(b);
            if b_inner.len() == 2 {
                let b_a = b_inner[0];
                let b_b = b_inner[1];
                if b_a.get_op_type() == OpType::Constant {
                    let v = self.op_add(a, b_a);
                    return self.op_add(v, b_b);
                } else if b_b.get_op_type() == OpType::Constant {
                    let v = self.op_add(a, b_b);
                    return self.op_add(v, b_a);
                }
            }
        } else if b_type == OpType::Constant && a_type == OpType::Add {
            return self.simplify_add(b, a);
        }
        if a_type == OpType::Constant && a.get_constant_value() == 0 {
            return b;
        }
        if b_type == OpType::Constant && b.get_constant_value() == 0 {
            return a;
        }
        let value = SymFeltRefValue {
            op_type: OpType::Add,
            const_param: 0,
            inputs: vec![a, b],
        };
        self.store.insert(value)
    }

    fn op_std_binary_op(&mut self, op_type: OpType, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        let a_type = a.get_op_type();
        let b_type = b.get_op_type();
        if (a_type == OpType::Constant
            || a_type == OpType::ConstantTrue
            || a_type == OpType::ConstantFalse)
            && (b_type == OpType::Constant
                || b_type == OpType::ConstantTrue
                || b_type == OpType::ConstantFalse)
        {
            let a_val = a.get_constant_value();
            let b_val = b.get_constant_value();
            return self.op_const(op_type.eval_binary_constant(a_val, b_val));
        }
        if (op_type == OpType::Add || op_type == OpType::Sub)
            && b_type == OpType::Constant
            && b.get_constant_value() == 0
        {
            return a;
        }
        if op_type == OpType::Mul
            && (a_type == OpType::Constant && a.get_constant_value() == 0
                || b_type == OpType::Constant && b.get_constant_value() == 0)
        {
            return self.op_const(0);
        }
        // if op_type == OpType::Add {
        //     return self.simplify_add(a, b)
        // }
        let value = SymFeltRefValue {
            op_type,
            const_param: 0,
            inputs: vec![a, b],
        };
        self.store.insert(value)
    }
    fn op_std_binary_op_u32(
        &mut self,
        op_type: OpType,
        a: SymFeltRef,
        b: SymFeltRef,
    ) -> SymFeltRef {
        let a_type = a.get_op_type();
        let b_type = b.get_op_type();
        if (a_type == OpType::Constant
            || a_type == OpType::ConstantTrue
            || a_type == OpType::ConstantFalse)
            && (b_type == OpType::Constant
                || b_type == OpType::ConstantTrue
                || b_type == OpType::ConstantFalse)
        {
            let a_val = a.get_constant_value();
            let b_val = b.get_constant_value();
            return self.op_const(op_type.eval_binary_constant(a_val, b_val));
        }
        let a_u32 = self.op_cast_u32(a);
        let b_u32 = self.op_cast_u32(b);
        let value = SymFeltRefValue {
            op_type,
            const_param: 0,
            inputs: vec![a_u32, b_u32],
        };
        self.store.insert(value)
    }
    fn op_std_unary_op(&mut self, op_type: OpType, a: SymFeltRef) -> SymFeltRef {
        let a_type = a.get_op_type();
        if a_type == OpType::Constant
            || a_type == OpType::ConstantTrue
            || a_type == OpType::ConstantFalse
        {
            let a_val = a.get_constant_value();
            return self.op_const(op_type.eval_unary_constant(a_val));
        }
        let value = SymFeltRefValue {
            op_type,
            const_param: 0,
            inputs: vec![a],
        };
        self.store.insert(value)
    }

    fn op_valueless(&mut self, op_type: OpType) -> SymFeltRef {
        SymFeltRef::new_valueless(op_type)
    }

    fn op_target_at(&mut self, parent: SymFeltRef, index: u64) -> SymFeltRef {
        let value = SymFeltRefValue {
            op_type: OpType::TargetAt,
            const_param: index,
            inputs: vec![parent, SymFeltRef::new_constant(index)],
        };
        self.store.insert(value)
    }

    fn op_target_at_vec(&mut self, parent: SymFeltRef, length: u64) -> Vec<SymFeltRef> {
        (0..length).map(|i| self.op_target_at(parent, i)).collect()
    }

    fn op_target_at_array<const N: usize>(&mut self, parent: SymFeltRef) -> [SymFeltRef; N] {
        core::array::from_fn(|i| self.op_target_at(parent, i as u64))
    }

    fn create_contract_state_ref(
        &mut self,
        contract_state_tree_height: u16,
        contract_id: SymFeltRef,
        user_id: SymFeltRef,
        index: SymFeltRef,
    ) -> SymFeltRef {
        let value = SymFeltRefValue {
            op_type: OpType::GetStateQueryResultSingle,
            const_param: ((contract_state_tree_height as u64) << 48)
                | (self.external_function_call_count as u64) << 32
                | (self.set_state_command_count as u64),
            inputs: vec![contract_id, user_id, index],
        };
        self.store.insert(value)
    }

    fn create_self_contract_state_ref(&mut self, index: SymFeltRef) -> SymFeltRef {
        self.create_contract_state_ref(
            self.contract_state_tree_height,
            SymFeltRef::new_valueless(OpType::GetContractId),
            SymFeltRef::new_valueless(OpType::GetUserId),
            index,
        )
    }

    fn cset_felt(&mut self, old_value: SymFeltRef, new_value: SymFeltRef) -> SymFeltRef {
        if self.condition_stack.is_empty() {
            new_value
        } else {
            let op_type = self.current_condition.get_op_type();
            if op_type == OpType::ConstantTrue {
                new_value
            } else if op_type == OpType::ConstantFalse {
                old_value
            } else {
                let condition = self.current_condition;
                self.op_select(condition, new_value, old_value)
            }
        }
    }
}

pub trait ToFelts<F: ContextFelt>: Clone {
    fn to_felts(&self) -> Vec<F>;
    fn from_felts(felts: &[F]) -> Self;
}

impl<F: ContextFelt> ToFelts<F> for F {
    fn to_felts(&self) -> Vec<F> {
        vec![*self]
    }

    fn from_felts(felts: &[F]) -> Self {
        felts[0]
    }
}

impl Context<SymFeltRef> for ExecContext {
    fn add_input(&mut self) -> SymFeltRef {
        let input = SymFeltRef::new_input(self.input_count);
        self.input_count += 1;
        input
    }
    fn add_inputs(&mut self, count: u64) -> Vec<SymFeltRef> {
        (0..count).map(|_| self.add_input()).collect()
    }

    fn op_cast_u32(&mut self, a: SymFeltRef) -> SymFeltRef {
        let op_type = a.get_op_type();
        if op_type.get_data_type() == DPNBuiltInDataType::U32Target {
            a
        } else if op_type == OpType::Constant
            || op_type == OpType::ConstantTrue
            || op_type == OpType::ConstantFalse
        {
            let value = a.get_constant_value();
            self.op_const(value & 0xFFFFFFFFu64)
        } else {
            let value = SymFeltRefValue {
                op_type: OpType::CastU32,
                const_param: 0,
                inputs: vec![a],
            };
            self.store.insert(value)
        }
    }

    fn op_const(&mut self, value: u64) -> SymFeltRef {
        SymFeltRef::new_constant(value)
    }

    fn op_bool(&mut self, value: bool) -> SymFeltRef {
        SymFeltRef::new_valueless(if value {
            OpType::ConstantTrue
        } else {
            OpType::ConstantFalse
        })
    }

    fn op_select(&mut self, condition: SymFeltRef, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        let condition_type = condition.get_op_type();
        if a.eq(&b) {
            a
        } else if condition_type == OpType::ConstantTrue {
            a
        } else if condition_type == OpType::ConstantFalse {
            b
        } else if condition_type == OpType::Constant {
            let condition_val = condition.get_constant_value();
            if condition_val == 0 {
                b
            } else {
                a
            }
        } else {
            let value = SymFeltRefValue {
                op_type: OpType::Select,
                const_param: 0,
                inputs: vec![condition, a, b],
            };
            self.store.insert(value)
        }
    }

    fn op_add(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Add, a, b)
    }

    fn op_sub(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Sub, a, b)
    }

    fn op_mul(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Mul, a, b)
    }

    fn op_div(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Div, a, b)
    }

    fn op_mod(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Mod, a, b)
    }

    fn op_bool_not(&mut self, a: SymFeltRef) -> SymFeltRef {
        self.op_std_unary_op(OpType::BoolNot, a)
    }

    fn op_bool_and(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::BoolAnd, a, b)
    }

    fn op_bool_or(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::BoolOr, a, b)
    }

    fn op_bool_or_many(&mut self, values: &[SymFeltRef]) -> SymFeltRef {
        let mut result = values[0];
        for i in 1..values.len() {
            result = self.op_bool_or(result, values[i]);
        }
        result
    }
    fn op_bool_and_many(&mut self, values: &[SymFeltRef]) -> SymFeltRef {
        let mut result = values[0];
        for i in 1..values.len() {
            result = self.op_bool_and(result, values[i]);
        }
        result
    }

    fn op_neg(&mut self, a: SymFeltRef) -> SymFeltRef {
        self.op_std_unary_op(OpType::UnaryNegative, a)
    }

    fn op_inverse(&mut self, a: SymFeltRef) -> SymFeltRef {
        self.op_std_unary_op(OpType::UnaryInverse, a)
    }

    fn op_exp(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Exp, a, b)
    }
    fn op_eq(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Eq, a, b)
    }

    fn op_neq(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        let eq = self.op_std_binary_op(OpType::Eq, a, b);
        self.op_bool_not(eq)
    }
    fn op_lt(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Lt, a, b)
    }

    fn op_lte(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Lte, a, b)
    }

    fn op_gt(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Gt, a, b)
    }

    fn op_gte(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Gte, a, b)
    }

    // start u32 ops
    fn op_u32_xor(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op_u32(OpType::U32Xor, a, b)
    }
    fn op_u32_or(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op_u32(OpType::U32Or, a, b)
    }
    fn op_u32_and(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op_u32(OpType::U32And, a, b)
    }
    fn op_u32_shl(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op_u32(OpType::U32ShiftLeft, a, b)
    }
    fn op_u32_shr(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op_u32(OpType::U32ShiftRight, a, b)
    }

    // end u32 ops
    fn op_true(&mut self) -> SymFeltRef {
        self.op_bool(true)
    }

    fn op_false(&mut self) -> SymFeltRef {
        self.op_bool(false)
    }

    fn assert_eq(&mut self, left: SymFeltRef, right: SymFeltRef, message: &'static str) {
        self.assertions.push(SymRefAssertion {
            left,
            right,
            message,
        });
    }

    fn assert_true(&mut self, left: SymFeltRef, message: &'static str) {
        self.assert_eq(
            left,
            SymFeltRef::new_valueless(OpType::ConstantTrue),
            message,
        );
    }

    fn start_if_block(&mut self, condition: SymFeltRef) {
        self.condition_stack.push(IfConditionStack {
            conditions: vec![condition],
            current_condition: condition,
        });
        self.current_condition = self.resolve_current_condition();
    }

    fn start_else_if_block(&mut self, condition: SymFeltRef) {
        if self.condition_stack.is_empty() {
            panic!("Cannot add else if block without starting an if block first");
        }
        let last_conditions = self.condition_stack.last().unwrap().conditions.clone();
        let one_of_prev_true = self.op_bool_or_many(&last_conditions);
        let all_prev_not_true = self.op_bool_not(one_of_prev_true);
        let new_condition = self.op_bool_and(all_prev_not_true, condition);
        self.condition_stack
            .last_mut()
            .unwrap()
            .conditions
            .push(condition);
        self.condition_stack.last_mut().unwrap().current_condition = new_condition;
        self.current_condition = self.resolve_current_condition();
    }

    fn start_else_block(&mut self) {
        if self.condition_stack.is_empty() {
            panic!("Cannot add else block without starting an if block first");
        }
        let last_conditions = self.condition_stack.last().unwrap().conditions.clone();
        let one_of_prev_true = self.op_bool_or_many(&last_conditions);
        let all_prev_not_true = self.op_bool_not(one_of_prev_true);
        self.condition_stack.last_mut().unwrap().current_condition = all_prev_not_true;
        self.current_condition = self.resolve_current_condition();
    }

    fn end_if_block(&mut self) {
        if self.condition_stack.is_empty() {
            panic!("Cannot end if block without starting an if block first");
        }
        self.condition_stack.pop();
    }

    fn resolve_current_condition(&mut self) -> SymFeltRef {
        if self.condition_stack.is_empty() {
            self.op_true()
        } else {
            let conditions = self
                .condition_stack
                .iter()
                .map(|x| x.current_condition)
                .collect::<Vec<_>>();
            self.op_bool_and_many(&conditions)
        }
    }

    fn cset<V: ToFelts<SymFeltRef>>(&mut self, old_value: V, new_value: V) -> V {
        if self.condition_stack.is_empty() {
            new_value
        } else {
            let old_felts = old_value.to_felts();
            let new_felts = new_value.to_felts();
            let result_felts = old_felts
                .into_iter()
                .zip(new_felts.into_iter())
                .map(|(old, new)| self.cset_felt(old, new))
                .collect::<Vec<_>>();
            V::from_felts(&result_felts)
        }
    }

    fn cset_str<V: ToFelts<SymFeltRef>>(
        &mut self,
        left: &'static str,
        old_value: V,
        new_value: V,
    ) -> V {
        println!("cset_str: {}", left);
        self.cset(old_value, new_value)
    }

    fn pop_condition(&mut self) {
        self.condition_stack.pop();
    }

    fn hash(&mut self, values: &[SymFeltRef]) -> [SymFeltRef; 4] {
        let op = SymFeltRefValue {
            op_type: OpType::HashNoPad,
            const_param: 0,
            inputs: values.to_vec(),
        };
        let parent = self.store.insert(op);
        self.op_target_at_array::<4>(parent)
    }

    fn split_bits(&mut self, value: SymFeltRef, num_bits: u64) -> Vec<SymFeltRef> {
        let op = SymFeltRefValue {
            op_type: OpType::SplitBits,
            const_param: num_bits,
            inputs: vec![value, SymFeltRef::new_constant(num_bits)],
        };
        let parent = self.store.insert(op);
        self.op_target_at_vec(parent, num_bits)
    }
    fn sum_bits(&mut self, bits: &[SymFeltRef]) -> SymFeltRef {
        let op = SymFeltRefValue {
            op_type: OpType::SumBits,
            const_param: 0,
            inputs: bits.to_vec(),
        };
        self.store.insert(op)
    }

    fn get_user_id(&mut self) -> SymFeltRef {
        SymFeltRef::new_valueless(OpType::GetUserId)
    }
    fn get_contract_id(&mut self) -> SymFeltRef {
        SymFeltRef::new_valueless(OpType::GetContractId)
    }
    fn get_checkpoint_id(&mut self) -> SymFeltRef {
        SymFeltRef::new_valueless(OpType::GetCheckpointId)
    }
    fn get_last_nonce(&mut self) -> SymFeltRef {
        SymFeltRef::new_valueless(OpType::GetNonce)
    }

    fn get_user_public_key_hash(&mut self) -> [SymFeltRef; 4] {
        self.op_target_at_array(SymFeltRef::new_valueless(OpType::GetUserPublicKeyHash))
    }

    fn op_get_state_felt(
        &mut self,
        contract_state_tree_height: u16,
        contract_id: SymFeltRef,
        user_id: SymFeltRef,
        index: SymFeltRef,
    ) -> SymFeltRef {
        self.create_contract_state_ref(contract_state_tree_height, contract_id, user_id, index)
    }

    fn get_value(&mut self, a: SymFeltRef) -> u64 {
        a.get_constant_value()
    }

    fn get_bool_value(&mut self, a: SymFeltRef) -> bool {
        a.get_constant_bool_value_multi()
    }

    fn op_set_state_felt(&mut self, index: SymFeltRef, value: SymFeltRef) -> SymFeltRef {
        let core_ref = self.create_self_contract_state_ref(index);
        if core_ref.eq(&value) {
            return value;
        }
        let set_sym = SetSymFeltRef::new(self.external_function_call_count, index, value);
        self.set_state_commands.push(set_sym);
        self.set_state_command_count += 1;
        value
    }

    fn op_set_state_obj<T: ToFelts<SymFeltRef>>(&mut self, index: SymFeltRef, value: T) -> T {
        let felts = value.to_felts();
        let existing_refs = felts
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                let ind = self.op_add(index, SymFeltRef::new_constant(i as u64));
                let v = self.create_self_contract_state_ref(ind);
                (v, x)
            })
            .collect::<Vec<_>>();
        for (old_value, new_value) in existing_refs {
            if old_value.eq(&new_value) {
                continue;
            }
            let set_sym =
                SetSymFeltRef::new(self.external_function_call_count, old_value, new_value);
            self.set_state_commands.push(set_sym);
            self.set_state_command_count += 1;
        }
        value
    }

    fn cset_state<V: ToFelts<SymFeltRef>>(&mut self, old_value: V, new_value: V) -> V {
        let old_felts = old_value.to_felts();
        for old in old_felts.iter() {
            if old.get_op_type() != OpType::GetStateQueryResultSingle {
                panic!("cset_state can only be used with state objects");
            }
        }
        let start_index = self.store.get_direct_children(old_felts[0])[2];
        self.op_set_state_obj(start_index, new_value)
    }
}
