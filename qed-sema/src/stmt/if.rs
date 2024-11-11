use qed_ast::ExprId;

use crate::{stmt::block::CheckedBlockNode, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedCase {
    pub predicate: ExprId,
    pub type_id: TypeId,
    pub body: CheckedBlockNode,
}

impl CheckedCase {
    pub fn new(predicate: ExprId, type_id: TypeId, body: CheckedBlockNode) -> Self {
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
    pub else_branch: Option<CheckedBlockNode>,
}
