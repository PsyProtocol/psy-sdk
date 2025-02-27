use crate::{ExprId, NodeInfo, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub predicate: ExprId,
    pub body: StmtId,
}

impl Case {
    pub fn new(predicate: ExprId, body: StmtId) -> Self {
        Self { predicate, body }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExprNode {
    pub if_branch: Case,
    pub elseif_branches: Vec<Case>,
    pub else_branch: Option<StmtId>,
}

impl NodeInfo for IfExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::IfExpr
    }
}
