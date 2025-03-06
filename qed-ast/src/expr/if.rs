use crate::{ExprId, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub predicate: ExprId,
    pub body: ExprId,
}

impl Case {
    pub fn new(predicate: ExprId, body: ExprId) -> Self {
        Self { predicate, body }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExprNode {
    pub if_branch: Case,
    pub elseif_branches: Vec<Case>,
    pub else_branch: Option<ExprId>,
}

impl NodeInfo for IfExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::IfExpr
    }
}
