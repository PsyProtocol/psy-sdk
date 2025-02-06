use std::{
    cell::RefCell,
    collections::HashMap,
    ops::{Index, IndexMut},
    rc::Rc,
};

use either::Either;
use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;
use qed_ast::{ExprId, IdentId, NodeInfo, NodeType};
use qed_builder::{ContextFelt, ToFelts};
pub use strum::EnumTryAs;

use crate::{TypeId, BOOL_TYPE, FELT_TYPE};

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum CheckedValueNode<F> {
    Felt(F),
    Bool(F),
    Array(TypeId, Vec<ExprId>),
    Struct(TypeId, IndexMap<IdentId, ExprId>),
    Type(TypeId),
}

impl<F> NodeInfo for CheckedValueNode<F> {
    fn node_type(&self) -> NodeType {
        NodeType::ValueExpr
    }
}

#[derive(Debug, PartialEq, EnumAsInner)]
pub enum CheckedValue<F> {
    Felt(F),
    Bool(F),
    Array(TypeId, Vec<Rc<RefCell<CheckedValue<F>>>>),
    Struct(TypeId, IndexMap<IdentId, Rc<RefCell<CheckedValue<F>>>>),
    Type(TypeId),
}

impl<F: Clone> Clone for CheckedValue<F> {
    fn clone(&self) -> Self {
        match self {
            CheckedValue::Felt(f) => CheckedValue::Felt(f.clone()),
            CheckedValue::Bool(b) => CheckedValue::Bool(b.clone()),
            CheckedValue::Array(type_id, values) => CheckedValue::Array(
                type_id.clone(),
                values.iter().map(|x| Rc::clone(x)).collect(),
            ),
            CheckedValue::Struct(type_id, fields) => CheckedValue::Struct(
                type_id.clone(),
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), Rc::clone(v)))
                    .collect(),
            ),
            CheckedValue::Type(type_id) => CheckedValue::Type(type_id.clone()),
        }
    }
}

impl<F: Clone + From<u32> + ContextFelt> ToFelts<F> for CheckedValue<F> {
    fn to_felts(&self) -> Vec<F> {
        match self {
            CheckedValue::Felt(f) => vec![f.clone()],
            CheckedValue::Bool(b) => vec![b.clone()],
            CheckedValue::Array(type_id, values) => {
                let mut result = Vec::new();
                for value in values {
                    result.extend(value.borrow().to_felts());
                }
                result
            }
            CheckedValue::Struct(type_id, fields) => {
                let mut result = Vec::new();
                for (_, value) in fields {
                    result.extend(value.borrow().to_felts());
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
    type Output = Rc<RefCell<CheckedValue<F>>>;

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

impl<F: Clone> CheckedValue<F> {
    pub fn to_felt(&self) -> F {
        match self {
            CheckedValue::Felt(f) => f.clone(),
            _ => panic!("Expected felt value"),
        }
    }

    pub fn to_bool(&self) -> F {
        match self {
            CheckedValue::Bool(f) => f.clone(),
            _ => panic!("Expected bool value"),
        }
    }

    pub fn type_id(&self) -> TypeId {
        match self {
            CheckedValue::Felt(_) => FELT_TYPE,
            CheckedValue::Bool(_) => BOOL_TYPE,
            CheckedValue::Array(type_id, _) => type_id.clone(),
            CheckedValue::Struct(type_id, _) => type_id.clone(),
            CheckedValue::Type(type_id) => type_id.clone(),
        }
    }

    pub fn set_path(
        &mut self,
        path: &[usize],
        mut value: Rc<RefCell<CheckedValue<F>>>,
    ) -> anyhow::Result<()> {
        if !self.is_array() && !self.is_struct() {
            assert!(path.is_empty());
            std::mem::swap(self, &mut *value.borrow_mut());
            return Ok(());
        } else if path.is_empty() {
            std::mem::swap(self, &mut *value.borrow_mut());
            return Ok(());
        }

        match self {
            CheckedValue::Array(_, arr) => {
                let index = path[0];
                let rest = &path[1..];
                if let Some(inner) = arr.get_mut(index) {
                    inner.borrow_mut().set_path(rest, value)?;
                }
            }
            CheckedValue::Struct(_, map) => {
                let key = IdentId(path[0]);
                let rest = &path[1..];
                if let Some(inner) = map.get_mut(&key) {
                    inner.borrow_mut().set_path(rest, value)?;
                }
            }
            _ => {
                unreachable!()
            }
        }
        Ok(())
    }
}
