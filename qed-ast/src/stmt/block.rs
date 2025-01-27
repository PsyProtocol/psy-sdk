use crate::{AstVisitor, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockNode {
    pub stmts: Vec<StmtId>,
}

impl BlockNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::BlockStmt
    }
}
