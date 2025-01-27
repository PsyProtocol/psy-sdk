use qed_ast::{AssignmentOperator, ExprId, NodeType};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedAssignmentNode {
    pub variable: ExprId,
    pub operator: AssignmentOperator,
    pub value: ExprId,
    pub type_id: TypeId,
}

impl CheckedAssignmentNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::AssignmentStmt
    }
}
