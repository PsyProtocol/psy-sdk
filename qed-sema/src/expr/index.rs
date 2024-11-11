use qed_ast::{ExprId, IdentId};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedIndexAccessNode {
    pub value: ExprId,
    pub index: usize,
    pub type_id: TypeId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedMemberAccessNode {
    pub value: ExprId,
    pub field: IdentId,
    pub type_id: TypeId,
}
