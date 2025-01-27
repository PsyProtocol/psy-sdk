use crate::{AstVisitor, BlockNode, ExprId, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct WhileNode {
    pub predicate: ExprId,
    pub body: StmtId,
}

impl WhileNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::WhileStmt
    }
}
