use crate::{ExprId, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct TupleExprNode {
    pub elements: Vec<ExprId>,
}

impl NodeInfo for TupleExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::TupleExpr
    }
}
