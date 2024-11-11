use crate::{
    felt::sym_felt::{SymFeltRefValue, SymFeltStore, SymRefAssertion},
    ops::OpType,
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
    condition_stack: Vec<IfConditionStack>,
    current_condition: SymFeltRef,
}

impl ExecContext {
    pub fn new() -> Self {
        ExecContext {
            store: SymFeltStore::new(),
            input_count: 0,
            assertions: vec![],
            condition_stack: vec![],
            current_condition: SymFeltRef::new_valueless(OpType::ConstantTrue),
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
            return a;
        }
        if op_type == OpType::Add {
            return self.simplify_add(a, b);
        }
        let value = SymFeltRefValue {
            op_type,
            const_param: 0,
            inputs: vec![a, b],
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

    fn op_true(&mut self) -> SymFeltRef {
        self.op_bool(true)
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

    fn op_neg(&mut self, a: SymFeltRef) -> SymFeltRef {
        self.op_std_unary_op(OpType::Neg, a)
    }

    fn op_inverse(&mut self, a: SymFeltRef) -> SymFeltRef {
        self.op_std_unary_op(OpType::Inverse, a)
    }

    fn op_eq(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::Eq, a, b)
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

    fn op_bit_shr(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::BitShr, a, b)
    }

    fn op_bit_shl(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::BitShl, a, b)
    }

    fn op_bit_and(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::BitAnd, a, b)
    }

    fn op_bit_or(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::BitOr, a, b)
    }

    fn op_bit_xor(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(OpType::BitXor, a, b)
    }

    fn op_hash(&mut self, values: &[SymFeltRef]) -> [SymFeltRef; 4] {
        let op = SymFeltRefValue {
            op_type: OpType::HashNoPad,
            const_param: 0,
            inputs: values.to_vec(),
        };
        let parent = self.store.insert(op);
        self.op_target_at_array::<4>(parent)
    }

    fn assert_eq(&mut self, left: SymFeltRef, right: SymFeltRef) {
        self.assertions.push(SymRefAssertion { left, right });
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

    fn op_target_at(&mut self, parent: SymFeltRef, index: u64) -> SymFeltRef {
        let value = SymFeltRefValue {
            op_type: OpType::TargetAt,
            const_param: index,
            inputs: vec![parent, SymFeltRef::new_constant(index)],
        };
        self.store.insert(value)
    }

    fn get_value(&mut self, a: SymFeltRef) -> u64 {
        a.get_constant_value()
    }

    fn get_bool_value(&mut self, a: SymFeltRef) -> bool {
        a.get_constant_bool_value_multi()
    }
}
