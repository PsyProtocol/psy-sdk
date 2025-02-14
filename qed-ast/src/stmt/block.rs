use crate::{AstVisitor, NodeInfo, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockNode {
    pub stmts: Vec<StmtId>,
    pub uses: Vec<StmtId>,
}

impl NodeInfo for BlockNode {
    fn node_type(&self) -> NodeType {
        NodeType::BlockStmt
    }
}
