use crate::{ExprId, IdentId, NodeInfo, NodeType, Span};

#[derive(Clone, Debug, PartialEq)]
pub struct IndexAccessNode {
    pub target: ExprId,
    pub index: ExprId,
    pub span: Span,
}

impl NodeInfo for IndexAccessNode {
    fn node_type(&self) -> NodeType {
        NodeType::IndexAccessExpr
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemberAccessNode {
    pub target: ExprId,
    pub field: IdentId,
    pub span: Span,
}

impl NodeInfo for MemberAccessNode {
    fn node_type(&self) -> NodeType {
        NodeType::MemberAccessExpr
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TupleAccessNode {
    pub target: ExprId,
    pub index: usize,
    pub span: Span,
}

impl NodeInfo for TupleAccessNode {
    fn node_type(&self) -> NodeType {
        NodeType::TupleAccessExpr
    }
}
