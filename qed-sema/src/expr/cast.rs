use crate::{ExprId, TypeId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedCastNode {
    pub value: ExprId,
    pub target_type: TypeId,
}
