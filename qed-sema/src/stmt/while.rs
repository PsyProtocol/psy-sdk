use qed_ast::{ExprId, NodeInfo, NodeType, Span};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedWhileNode {
    pub predicate: ExprId,
    pub type_id: TypeId,
    pub body: ExprId,
    pub span: Span,
}

impl NodeInfo for CheckedWhileNode {
    fn node_type(&self) -> NodeType {
        NodeType::WhileStmt
    }
}
