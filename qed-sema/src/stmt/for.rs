use qed_ast::Location;

use crate::{ExprId, Identifier, NodeInfo, NodeType, ScopeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedForNode {
    pub variable: Identifier,
    pub start: ExprId,
    pub end: ExprId,
    pub body: ExprId,
    pub scope_id: ScopeId,
    pub location: Location,
}

impl NodeInfo for CheckedForNode {
    fn node_type(&self) -> NodeType {
        NodeType::ForStmt
    }
}
