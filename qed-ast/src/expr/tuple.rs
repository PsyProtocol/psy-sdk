use crate::{ExprId, NodeInfo, NodeType, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct TupleExprNode {
    pub elements: Vec<ExprId>,
    pub span: Span,
}

impl NodeInfo for TupleExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::TupleExpr
    }
}
