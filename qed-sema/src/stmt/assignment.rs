use qed_ast::{AssignmentOperator, ExprId, NodeInfo, NodeType};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedAssignmentNode {
    pub target: ExprId,
    pub operator: AssignmentOperator,
    pub value: ExprId,
    pub type_id: TypeId,
}

impl NodeInfo for CheckedAssignmentNode {
    fn node_type(&self) -> NodeType {
        NodeType::AssignmentStmt
    }
}
