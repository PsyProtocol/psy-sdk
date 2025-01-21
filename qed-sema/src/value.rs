use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use either::Either;
use qed_ast::{ExprId, IdentId};
pub use strum::{EnumIs, EnumTryAs};

use crate::{TypeId, BOOL_TYPE, FELT_TYPE};

#[derive(Clone, Debug, PartialEq, EnumIs, EnumTryAs)]
pub enum CheckedValueNode<F> {
    Felt(F),
    Bool(F),
    Array(TypeId, Vec<ExprId>),
    Struct(TypeId, HashMap<IdentId, ExprId>),
    Type(TypeId),
}

#[derive(Clone, Debug, PartialEq, EnumIs, EnumTryAs)]
pub enum CheckedValue<F> {
    Felt(F),
    Bool(F),
    Array(TypeId, Vec<CheckedValue<F>>),
    Struct(TypeId, HashMap<IdentId, CheckedValue<F>>),
    Type(TypeId),
}

impl<F> Index<usize> for CheckedValue<F> {
    type Output = CheckedValue<F>;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            CheckedValue::Array(_, values) => &values[index],
            _ => panic!("Indexing non-array value"),
        }
    }
}

impl<F> IndexMut<usize> for CheckedValue<F> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match self {
            CheckedValue::Array(_, values) => &mut values[index],
            _ => panic!("Indexing non-array value"),
        }
    }
}

impl<F> CheckedValue<F> {
    pub fn type_id(&self) -> TypeId {
        match self {
            CheckedValue::Felt(_) => FELT_TYPE,
            CheckedValue::Bool(_) => BOOL_TYPE,
            CheckedValue::Array(type_id, _) => type_id.clone(),
            CheckedValue::Struct(type_id, _) => type_id.clone(),
            CheckedValue::Type(type_id) => type_id.clone(),
        }
    }

    pub fn get_field(&self, field: IdentId) -> Option<&CheckedValue<F>> {
        match self {
            CheckedValue::Struct(_, fields) => fields.get(&field),
            _ => None,
        }
    }

    pub fn get_mut_field(&mut self, field: IdentId) -> Option<&mut CheckedValue<F>> {
        match self {
            CheckedValue::Struct(_, fields) => fields.get_mut(&field),
            _ => None,
        }
    }

    pub fn set_field(&mut self, field: IdentId, value: CheckedValue<F>) {
        match self {
            CheckedValue::Struct(_, fields) => {
                fields.insert(field, value);
            }
            _ => panic!("Setting field on non-struct value"),
        }
    }
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
