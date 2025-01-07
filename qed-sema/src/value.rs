use std::collections::HashMap;

use qed_ast::{ExprId, IdentId};
pub use strum::{EnumIs, EnumTryAs};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq, EnumIs, EnumTryAs)]
pub enum CheckedValueNode<F> {
    Felt(F),
    Bool(F),
    Array(TypeId, usize, Vec<ExprId>),
    Struct(TypeId, HashMap<IdentId, ExprId>),
    Type(TypeId),
}
