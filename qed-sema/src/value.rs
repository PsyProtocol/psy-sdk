use std::collections::HashMap;

use either::Either;
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

#[derive(Clone, Debug, PartialEq, EnumIs, EnumTryAs)]
pub enum CheckedValue<F> {
    Felt(F),
    Bool(F),
    Array(TypeId, usize, Vec<CheckedValue<F>>),
    Struct(TypeId, HashMap<IdentId, CheckedValue<F>>),
    Type(TypeId),
}

pub type CheckedValueOrNode<F> = Either<CheckedValueNode<F>, CheckedValue<F>>;

impl<F> From<CheckedValueNode<F>> for CheckedValueOrNode<F> {
    fn from(value: CheckedValueNode<F>) -> Self {
        CheckedValueOrNode::Left(value)
    }
}

impl<F> From<CheckedValue<F>> for CheckedValueOrNode<F> {
    fn from(value: CheckedValue<F>) -> Self {
        CheckedValueOrNode::Right(value)
    }
}
