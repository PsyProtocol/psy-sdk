use crate::{ExprId, NodeInfo, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct WhileNode {
    pub predicate: ExprId,
    pub body: StmtId,
}

impl NodeInfo for WhileNode {
    fn node_type(&self) -> NodeType {
        NodeType::WhileStmt
    }
}
