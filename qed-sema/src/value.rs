use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use either::Either;
use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;
use qed_ast::{ExprId, IdentId, NodeType};
use qed_builder::{ContextFelt, ToFelts};
pub use strum::EnumTryAs;

use crate::{TypeId, BOOL_TYPE, FELT_TYPE};

#[derive(Clone, Debug, PartialEq, EnumAsInner, EnumTryAs)]
pub enum CheckedValueNode<F> {
    Felt(F),
    Bool(F),
    Array(TypeId, Vec<ExprId>),
    Struct(TypeId, IndexMap<IdentId, ExprId>),
    Type(TypeId),
}

impl<F> CheckedValueNode<F> {
    pub fn node_type(&self) -> NodeType {
        NodeType::ValueExpr
        // match self {
        //     CheckedValueNode::Felt(_) => NodeType::FeltValue,
        //     CheckedValueNode::Bool(_) => NodeType::BoolValue,
        //     CheckedValueNode::Array(_, _) => NodeType::ArrayValue,
        //     CheckedValueNode::Struct(_, _) => NodeType::StructValue,
        //     CheckedValueNode::Type(_) => NodeType::TypeValue,
        // }
    }
}

#[derive(Clone, Debug, PartialEq, EnumAsInner, EnumTryAs)]
pub enum CheckedValue<F> {
    Felt(F),
    Bool(F),
    Array(TypeId, Vec<CheckedValue<F>>),
    Struct(TypeId, IndexMap<IdentId, CheckedValue<F>>),
    Type(TypeId),
}

impl<F: Clone + From<u32> + ContextFelt> ToFelts<F> for CheckedValue<F> {
    fn to_felts(&self) -> Vec<F> {
        match self {
            CheckedValue::Felt(f) => vec![f.clone()],
            CheckedValue::Bool(b) => vec![b.clone()],
            CheckedValue::Array(type_id, values) => {
                let mut result = Vec::new();
                for value in values {
                    result.extend(value.to_felts());
                }
                result
            }
            CheckedValue::Struct(type_id, fields) => {
                let mut result = Vec::new();
                for (_, value) in fields {
                    result.extend(value.to_felts());
                }
                result
            }
            CheckedValue::Type(type_id) => {
                unreachable!()
            }
        }
    }

    fn from_felts(felts: &[F]) -> Self {
        todo!()
    }
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
