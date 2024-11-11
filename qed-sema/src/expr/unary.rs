use qed_ast::{ExprId, UnaryOperator};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedUnaryNode {
    pub operator: UnaryOperator,
    pub rhs: ExprId,
    pub type_id: TypeId,
}
