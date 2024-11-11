use qed_ast::{BinaryOperator, ExprId};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedBinaryNode {
    pub lhs: ExprId,
    pub operator: BinaryOperator,
    pub rhs: ExprId,
    pub type_id: TypeId,
}
