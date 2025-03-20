use crate::{ExprId, Location, NodeInfo, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExprNode {
    pub stmts: Vec<StmtId>,
    pub expr: Option<ExprId>,
    pub location: Location,
}

impl NodeInfo for BlockExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::BlockExpr
    }
}
