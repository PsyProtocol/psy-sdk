use qed_ast::ExprId;
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{CheckedProgram, CheckedValueRef, TypeCheckerVisitorContext};

pub trait Evaluator<F: Clone + From<u32> + ContextFelt, C> {
    fn evaluate_expr(
        &mut self,
        program: &CheckedProgram<F>,
        expr_id: ExprId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> CheckedValueRef<F>;
    fn to_constant_value(&mut self, value: CheckedValueRef<F>) -> u64;
    fn from_constant_value(&mut self, value: u32) -> CheckedValueRef<F>;
}
