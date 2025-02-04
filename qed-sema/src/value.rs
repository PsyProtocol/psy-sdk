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

    pub fn set_path(&mut self, path: &[usize], value: CheckedValue<F>) -> anyhow::Result<()> {
        if path.is_empty() {
            *self = value;
            return Ok(());
        }
        match self {
            CheckedValue::Array(_, vec) => {
                let index = path[0];
                let rest = &path[1..];
                if let Some(inner) = vec.get_mut(index) {
                    inner.set_path(rest, value)?;
                }
            }
            CheckedValue::Struct(_, map) => {
                let key = IdentId(path[0]);
                let rest = &path[1..];
                if let Some(inner) = map.get_mut(&key) {
                    inner.set_path(rest, value)?;
                }
            }
            _ => {
                assert!(path.is_empty());
                *self = value;
            }
        }
        Ok(())
    }

    pub fn get_path(&self, path: &[usize]) -> anyhow::Result<&CheckedValue<F>> {
        if path.is_empty() {
            return Ok(self);
        }
        match self {
            CheckedValue::Array(_, vec) => {
                let index = path[0];
                let rest = &path[1..];
                vec.get(index)
                    .ok_or_else(|| anyhow::anyhow!("Index out of bounds"))?
                    .get_path(rest)
            }
            CheckedValue::Struct(_, map) => {
                let key = IdentId(path[0]);
                let rest = &path[1..];
                map.get(&key)
                    .ok_or_else(|| anyhow::anyhow!("Field not found"))?
                    .get_path(rest)
            }
            _ => {
                assert!(path.is_empty());
                Ok(self)
            }
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
