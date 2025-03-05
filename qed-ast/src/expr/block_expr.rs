use crate::{ExprId, NodeInfo, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExprNode {
    pub stmts: Vec<StmtId>,
    pub expr: Option<ExprId>,
}

impl NodeInfo for BlockExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::BlockExpr
    }
}
