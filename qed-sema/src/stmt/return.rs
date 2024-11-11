use qed_ast::ExprId;

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedReturnNode {
    pub ret: Option<(ExprId, TypeId)>,
}
