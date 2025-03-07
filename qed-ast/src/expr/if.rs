use crate::{ExprId, NodeInfo, NodeType, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub predicate: ExprId,
    pub body: ExprId,
    pub span: Span,
}

impl Case {
    pub fn new(predicate: ExprId, body: ExprId, span: Span) -> Self {
        Self {
            predicate,
            body,
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExprNode {
    pub if_branch: Case,
    pub elseif_branches: Vec<Case>,
    pub else_branch: Option<ExprId>,
    pub span: Span,
}

impl NodeInfo for IfExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::IfExpr
    }
}
