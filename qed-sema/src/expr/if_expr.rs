use crate::TypeId;
use qed_ast::{ExprId, NodeInfo, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedCase {
    pub predicate: ExprId,
    pub type_id: TypeId,
    pub body: StmtId,
}

impl CheckedCase {
    pub fn new(predicate: ExprId, type_id: TypeId, body: StmtId) -> Self {
        Self {
            predicate,
            type_id,
            body,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedIfExprNode {
    pub if_branch: CheckedCase,
    pub elseif_branches: Vec<CheckedCase>,
    pub else_branch: Option<StmtId>,
    pub type_id: TypeId,
}

impl NodeInfo for CheckedIfExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::IfExpr
    }
}
