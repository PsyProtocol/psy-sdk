use crate::{AstVisitor, ExprId, IdentId};

#[derive(Clone, Debug, PartialEq)]
pub struct IndexAccessNode {
    pub value: ExprId,
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemberAccessNode {
    pub value: ExprId,
    pub field: IdentId,
}
