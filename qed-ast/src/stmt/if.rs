use crate::{AstVisitor, BlockNode, ExprId, NodeInfo, NodeType, StmtId};

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
pub struct IfNode {
    pub if_branch: Case,
    pub elseif_branch: Vec<Case>,
    pub else_branch: Option<StmtId>,
}

impl NodeInfo for IfNode {
    fn node_type(&self) -> NodeType {
        NodeType::IfStmt
    }
}
