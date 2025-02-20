use crate::{ExprId, IdentId, NodeInfo, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct ForNode {
    pub variable: IdentId,
    pub start: ExprId,
    pub end: ExprId,
    pub body: StmtId,
}

impl NodeInfo for ForNode {
    fn node_type(&self) -> NodeType {
        NodeType::ForStmt
    }
}
