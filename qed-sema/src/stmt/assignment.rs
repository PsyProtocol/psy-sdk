use qed_ast::{AssignmentOperator, ExprId};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedAssignmentNode {
    pub variable: ExprId,
    pub operator: AssignmentOperator,
    pub value: ExprId,
    pub type_id: TypeId,
}
