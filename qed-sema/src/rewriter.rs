use qed_ast::{DefId, ExprId, StmtId};
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::TypeChecker;

pub trait Rewriter {
    fn instantiate_impl(&self, impl_id: DefId) -> DefId;
    fn instantiate_function(&self, function_id: DefId) -> DefId;
    fn rewrite_stmt(&self, stmt_id: StmtId) -> StmtId;
    fn rewrite_expr(&self, expr_id: ExprId) -> ExprId;
}

impl<F: Clone + From<u32> + ContextFelt, C> Rewriter for TypeChecker<F, C> {
    fn instantiate_impl(&self, impl_id: DefId) -> DefId {
        todo!()
    }

    fn instantiate_function(&self, function_id: DefId) -> DefId {
        todo!()
    }

    fn rewrite_stmt(&self, stmt_id: StmtId) -> StmtId {
        todo!()
    }

    fn rewrite_expr(&self, expr_id: ExprId) -> ExprId {
        todo!()
    }
}
