use crate::dpn::ops::sym_felt::SymFeltRefValue;

use super::{op_types::{DPNBuiltInDataType, DPNOpType}, sym_felt::{SymFeltRef, SymRefAssertion}, sym_felt_store::SymFeltStore};

#[derive(Debug, Clone)]
pub struct IfConditionStack {
    pub conditions: Vec<SymFeltRef>,
    pub current_condition: SymFeltRef,
}
#[derive(Debug, Clone)]
pub struct QExecContext {
    pub store: SymFeltStore,
    pub input_count: u64,
    pub assertions: Vec<SymRefAssertion>,
    condition_stack: Vec<IfConditionStack>,
    current_condition: SymFeltRef,

}

impl QExecContext {
    pub fn new() -> Self {
        QExecContext {
            store: SymFeltStore::new(),
            input_count: 0,
            assertions: vec![],
            condition_stack: vec![],
            current_condition: SymFeltRef::new_valueless(DPNOpType::ConstantTrue),
        }
    }
    pub fn op_cast_u32(&mut self, a: SymFeltRef) -> SymFeltRef {
        let op_type = a.get_op_type();
        if op_type.get_data_type() == DPNBuiltInDataType::U32Target {
            a
        }else if op_type == DPNOpType::Constant || op_type == DPNOpType::ConstantTrue || op_type == DPNOpType::ConstantFalse {
            let value = a.get_constant_value();
            self.op_const(value&0xFFFFFFFFu64)
        }else{
            let value = SymFeltRefValue {
                op_type: DPNOpType::CastU32,
                const_param: 0,
                inputs: vec![a],
            };
            self.store.insert(value)
        }


    }
    fn op_std_binary_op(&mut self, op_type: DPNOpType, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        let a_type = a.get_op_type();
        let b_type = b.get_op_type();
        if (a_type == DPNOpType::Constant || a_type == DPNOpType::ConstantTrue || a_type == DPNOpType::ConstantFalse) && (b_type == DPNOpType::Constant || b_type == DPNOpType::ConstantTrue || b_type == DPNOpType::ConstantFalse) {
            let a_val = a.get_constant_value();
            let b_val = b.get_constant_value();
            return self.op_const(op_type.eval_binary_constant(a_val, b_val));
        }
        let value = SymFeltRefValue {
            op_type,
            const_param: 0,
            inputs: vec![a, b],
        };
        self.store.insert(value)
    }
    fn op_std_binary_op_u32(&mut self, op_type: DPNOpType, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        let a_type = a.get_op_type();
        let b_type = b.get_op_type();
        if (a_type == DPNOpType::Constant || a_type == DPNOpType::ConstantTrue || a_type == DPNOpType::ConstantFalse) && (b_type == DPNOpType::Constant || b_type == DPNOpType::ConstantTrue || b_type == DPNOpType::ConstantFalse) {
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
    fn op_std_unary_op(&mut self, op_type: DPNOpType, a: SymFeltRef) -> SymFeltRef {
        let a_type = a.get_op_type();
        if a_type == DPNOpType::Constant || a_type == DPNOpType::ConstantTrue || a_type == DPNOpType::ConstantFalse {
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
    fn op_valueless(&mut self, op_type: DPNOpType) -> SymFeltRef {
        SymFeltRef::new_valueless(op_type)
    }
    pub fn op_select(&mut self, condition: SymFeltRef, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        let condition_type = condition.get_op_type();
        if condition_type == DPNOpType::ConstantTrue {
            a
        }else if condition_type == DPNOpType::ConstantFalse {
            b
        }else if condition_type == DPNOpType::Constant {
            let condition_val = condition.get_constant_value();
            if condition_val == 0 {
                b
            }else{
                a
            }
        }else{
            let value = SymFeltRefValue {
                op_type: DPNOpType::Select,
                const_param: 0,
                inputs: vec![condition, a, b],
            };
            self.store.insert(value)
        }
    }
    pub fn op_const(&mut self, value: u64) -> SymFeltRef {
        SymFeltRef::new_constant(value)
    }
    pub fn op_bool_not(&mut self, a: SymFeltRef) -> SymFeltRef {
        self.op_std_unary_op(DPNOpType::BoolNot, a)
    }
    pub fn op_bool_and(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(DPNOpType::BoolAnd, a, b)
    }
    pub fn op_bool_or(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(DPNOpType::BoolOr, a, b)
    }
    pub fn op_bool_or_many(&mut self, values: &[SymFeltRef]) -> SymFeltRef {
        let mut result = values[0];
        for i in 1..values.len() {
            result = self.op_bool_or(result, values[i]);
        }
        result
    }
    pub fn op_bool_and_many(&mut self, values: &[SymFeltRef]) -> SymFeltRef {
        let mut result = values[0];
        for i in 1..values.len() {
            result = self.op_bool_and(result, values[i]);
        }
        result
    }
    pub fn op_add(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(DPNOpType::Add, a, b)
    }
    pub fn op_sub(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(DPNOpType::Sub, a, b)
    }
    pub fn op_mul(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(DPNOpType::Mul, a, b)
    }
    pub fn op_div(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(DPNOpType::Div, a, b)
    }
    pub fn op_mod(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(DPNOpType::Mod, a, b)
    }
    pub fn op_exp(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(DPNOpType::Exp, a, b)
    }
    pub fn op_eq(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(DPNOpType::Eq, a, b)
    }
    pub fn op_neq(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        let eq = self.op_std_binary_op(DPNOpType::Eq, a, b);
        self.op_bool_not(eq)
    }
    pub fn op_lt(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op(DPNOpType::Lt, a, b)
    }
    pub fn op_lte(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        let lt = self.op_std_binary_op(DPNOpType::Lt, a, b);
        let eq = self.op_std_binary_op(DPNOpType::Eq, a, b);
        self.op_bool_or(lt, eq)
    }
    pub fn op_gt(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        let lt = self.op_std_binary_op(DPNOpType::Lt, b, a);
        self.op_bool_not(lt)
    }
    pub fn op_gte(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        let lt = self.op_std_binary_op(DPNOpType::Lt, b, a);
        let eq = self.op_std_binary_op(DPNOpType::Eq, a, b);
        self.op_bool_or(lt, eq)
    }

    // start u32 ops
    pub fn op_u32_xor(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op_u32(DPNOpType::U32Xor, a, b)
    }
    pub fn op_u32_or(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op_u32(DPNOpType::U32Or, a, b)
    }
    pub fn op_u32_and(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op_u32(DPNOpType::U32And, a, b)
    }
    pub fn op_u32_shl(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op_u32(DPNOpType::U32ShiftLeft, a, b)
    }
    pub fn op_u32_shr(&mut self, a: SymFeltRef, b: SymFeltRef) -> SymFeltRef {
        self.op_std_binary_op_u32(DPNOpType::U32ShiftRight, a, b)
    }

    // end u32 ops
    pub fn op_true(&mut self) -> SymFeltRef {
        self.op_valueless(DPNOpType::ConstantTrue)
    }

    pub fn op_false(&mut self) -> SymFeltRef {
        self.op_valueless(DPNOpType::ConstantFalse)
    }

    pub fn add_input(&mut self) -> SymFeltRef {
        let input = SymFeltRef::new_input(self.input_count);
        self.input_count += 1;
        input
    }
    pub fn add_inputs(&mut self, count: u64) -> Vec<SymFeltRef> {
        (0..count).map(|_| self.add_input()).collect()
    }
    pub fn assert_eq(&mut self, left: SymFeltRef, right: SymFeltRef, message: &'static str) {
        self.assertions.push(SymRefAssertion { left, right, message });
    }
    pub fn assert_true(&mut self, left: SymFeltRef, message: &'static str) {
        self.assert_eq(left, SymFeltRef::new_valueless(DPNOpType::ConstantTrue), message);
    }
    pub fn cset(&mut self, old_value: SymFeltRef, new_value: SymFeltRef) -> SymFeltRef {
        if self.condition_stack.is_empty() {
            new_value
        }else{
            let op_type = self.current_condition.get_op_type();
            if op_type == DPNOpType::ConstantTrue {
                new_value
            }else if op_type == DPNOpType::ConstantFalse {
                old_value
            }else{
                let condition = self.current_condition;
                self.op_select(condition, new_value, old_value)
            }
        }
    }
    pub fn start_if_block(&mut self, condition: SymFeltRef) {
        self.condition_stack.push(IfConditionStack {
            conditions: vec![condition],
            current_condition: condition,
        });
        self.current_condition = self.resolve_current_condition();
    }
    pub fn start_else_if_block(&mut self, condition: SymFeltRef) {
        if !self.condition_stack.is_empty() {
            panic!("Cannot add else if block without starting an if block first");
        }
        let last_conditions = self.condition_stack.last().unwrap().conditions.clone();
        let all_prev_true = self.op_bool_and_many(&last_conditions);
        let all_prev_not_true = self.op_bool_not(all_prev_true);
        let new_condition = self.op_bool_and(all_prev_not_true, condition);
        self.condition_stack.last_mut().unwrap().conditions.push(condition);
        self.condition_stack.last_mut().unwrap().current_condition = new_condition;
        self.current_condition = self.resolve_current_condition();
    }
    pub fn start_else_block(&mut self) {
        if !self.condition_stack.is_empty() {
            panic!("Cannot add else if block without starting an if block first");
        }
        let last_conditions = self.condition_stack.last().unwrap().conditions.clone();
        let all_prev_true = self.op_bool_and_many(&last_conditions);
        let all_prev_not_true = self.op_bool_not(all_prev_true);
        self.condition_stack.last_mut().unwrap().current_condition = all_prev_not_true;
        self.current_condition = self.resolve_current_condition();
    }
    pub fn end_if_block(&mut self) {
        self.condition_stack.pop();
    }
    pub fn resolve_current_condition(&mut self) -> SymFeltRef {
        if self.condition_stack.is_empty() {
            self.op_true()
        }else{
            let conditions = self.condition_stack.iter().map(|x| x.current_condition).collect::<Vec<_>>();
            self.op_bool_and_many(&conditions)
        }
    }
    pub fn pop_condition(&mut self) {
        self.condition_stack.pop();
    }
    
}