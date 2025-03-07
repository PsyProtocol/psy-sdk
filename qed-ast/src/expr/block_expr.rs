use crate::{ExprId, NodeInfo, NodeType, Span, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExprNode {
    pub stmts: Vec<StmtId>,
    pub expr: Option<ExprId>,
    pub span: Span,
}

impl NodeInfo for BlockExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::BlockExpr
    }
}
