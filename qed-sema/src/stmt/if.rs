use qed_ast::{ExprId, NodeInfo, NodeType, StmtId};

use crate::{stmt::block::CheckedBlockNode, TypeId};

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
pub struct CheckedIfNode {
    pub if_branch: CheckedCase,
    pub elseif_branch: Vec<CheckedCase>,
    pub else_branch: Option<StmtId>,
}

impl NodeInfo for CheckedIfNode {
    fn node_type(&self) -> NodeType {
        NodeType::IfStmt
    }
}
