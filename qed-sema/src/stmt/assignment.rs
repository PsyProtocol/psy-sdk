use qed_ast::{AssignmentOperator, ExprId, NodeInfo, NodeType, Span};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedAssignmentNode {
    pub target: ExprId,
    pub operator: AssignmentOperator,
    pub value: ExprId,
    pub type_id: TypeId,
    pub span: Span,
}

impl NodeInfo for CheckedAssignmentNode {
    fn node_type(&self) -> NodeType {
        NodeType::AssignmentStmt
    }
}
