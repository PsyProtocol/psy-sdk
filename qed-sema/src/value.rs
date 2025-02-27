use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;
use qed_ast::{ExprId, IdentId, NodeInfo, NodeType};
use qedlang_core::dpn::ops::context_trait::{ContextFelt, ToFelts};

use crate::{Result, TypeId, BOOL_TYPE, FELT_TYPE, VOID_TYPE};

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

#[derive(Debug, EnumAsInner)]
pub enum CheckedValue<F> {
    Felt(F),
    Bool(F),
    Array(TypeId, Vec<CheckedValueRef<F>>),
    Struct(TypeId, IndexMap<IdentId, CheckedValueRef<F>>),
    Type(TypeId),
}

#[derive(Debug)]
pub struct CheckedValueRef<F>(Rc<RefCell<CheckedValue<F>>>);

impl<F: Clone> Clone for CheckedValueRef<F> {
    fn clone(&self) -> Self {
        match &*self.0.borrow() {
            CheckedValue::Felt(f) => {
                CheckedValueRef(Rc::new(RefCell::new(CheckedValue::Felt(f.clone()))))
            }
            CheckedValue::Bool(b) => {
                CheckedValueRef(Rc::new(RefCell::new(CheckedValue::Bool(b.clone()))))
            }
            CheckedValue::Array(_type_id, _values) => CheckedValueRef(self.0.clone()),
            CheckedValue::Struct(_type_id, _fields) => CheckedValueRef(self.0.clone()),
            CheckedValue::Type(type_id) => {
                CheckedValueRef(Rc::new(RefCell::new(CheckedValue::Type(type_id.clone()))))
            }
        }
    }
}

impl<F: Clone + PartialEq> PartialEq for CheckedValueRef<F> {
    fn eq(&self, other: &Self) -> bool {
        match (&*self.0.borrow(), &*other.0.borrow()) {
            (CheckedValue::Felt(f1), CheckedValue::Felt(f2)) => f1 == f2,
            (CheckedValue::Bool(b1), CheckedValue::Bool(b2)) => b1 == b2,
            (CheckedValue::Array(_, _a1), CheckedValue::Array(_, _a2)) => {
                std::ptr::eq(Rc::as_ptr(&self.as_rc()), Rc::as_ptr(&other.as_rc()))
            }
            (CheckedValue::Struct(_, _s1), CheckedValue::Struct(_, _s2)) => {
                std::ptr::eq(Rc::as_ptr(&self.as_rc()), Rc::as_ptr(&other.as_rc()))
            }
            (CheckedValue::Type(t1), CheckedValue::Type(t2)) => t1 == t2,
            _ => false,
        }
    }
}

impl<F: Clone + From<u32> + ContextFelt> ToFelts<F> for CheckedValueRef<F> {
    fn to_felts(&self) -> Vec<F> {
        match &*self.0.borrow() {
            CheckedValue::Felt(f) => vec![f.clone()],
            CheckedValue::Bool(b) => vec![b.clone()],
            CheckedValue::Array(_type_id, values) => {
                let mut result = Vec::new();
                for value in values {
                    result.extend(value.to_felts());
                }
                result
            }
            CheckedValue::Struct(_type_id, fields) => {
                let mut result = Vec::new();
                for (_, value) in fields {
                    result.extend(value.to_felts());
                }
                result
            }
            CheckedValue::Type(type_id) => match type_id {
                &VOID_TYPE => vec![],
                _ => unreachable!(),
            },
        }
    }

    fn from_felts(felts: &[F]) -> Self {
        if felts.is_empty() {
            panic!("from_felts: empty input");
        }
        if felts.len() == 1 {
            let value = felts[0].clone();
            return CheckedValueRef::new_rc(CheckedValue::Felt(value));
        }
        todo!()
    }
}

impl<F: Clone> CheckedValueRef<F> {
    pub fn new_rc(value: CheckedValue<F>) -> Self {
        Self(Rc::new(RefCell::new(value)))
    }

    pub fn as_rc(&self) -> Rc<RefCell<CheckedValue<F>>> {
        Rc::clone(&self.0)
    }

    pub fn borrow_mut(&self) -> RefMut<'_, CheckedValue<F>> {
        self.0.borrow_mut()
    }

    pub fn borrow(&self) -> Ref<'_, CheckedValue<F>> {
        self.0.borrow()
    }

    pub fn from_felt(value: F) -> Self {
        Self::new_rc(CheckedValue::Felt(value))
    }

    pub fn from_bool(value: F) -> Self {
        Self::new_rc(CheckedValue::Bool(value))
    }

    pub fn from_vec(type_id: TypeId, data: impl IntoIterator<Item = F>) -> Self {
        Self::new_rc(CheckedValue::Array(
            type_id,
            data.into_iter().map(CheckedValueRef::from_felt).collect(),
        ))
    }

    pub fn is_felt(&self) -> bool {
        self.0.borrow().is_felt()
    }

    pub fn is_bool(&self) -> bool {
        self.0.borrow().is_bool()
    }

    pub fn is_struct(&self) -> bool {
        self.0.borrow().is_struct()
    }

    pub fn is_array(&self) -> bool {
        self.0.borrow().is_array()
    }

    pub fn to_felt(&self) -> F {
        match &*self.0.borrow() {
            CheckedValue::Felt(f) => f.clone(),
            _ => panic!("Expected felt value"),
        }
    }

    pub fn to_bool(&self) -> F {
        match &*self.0.borrow() {
            CheckedValue::Bool(f) => f.clone(),
            _ => panic!("Expected bool value"),
        }
    }

    pub fn to_array<const N: usize>(&self) -> [F; N]
    where
        F: std::fmt::Debug,
    {
        self.to_vec().try_into().unwrap()
    }

    pub fn to_vec(&self) -> Vec<F> {
        match &*self.0.borrow() {
            CheckedValue::Array(_, arr) => arr.into_iter().map(|x| x.to_felt()).collect::<Vec<F>>(),
            _ => panic!("Expected array value"),
        }
    }

    pub fn type_id(&self) -> TypeId {
        match &*self.0.borrow() {
            CheckedValue::Felt(_) => FELT_TYPE,
            CheckedValue::Bool(_) => BOOL_TYPE,
            CheckedValue::Array(type_id, _) => type_id.clone(),
            CheckedValue::Struct(type_id, _) => type_id.clone(),
            CheckedValue::Type(type_id) => type_id.clone(),
        }
    }

    pub fn set_path(&mut self, path: &[usize], value: Self) -> Result<()> {
        if path.is_empty() {
            *self = value.clone();
            return Ok(());
        }

        match &mut *self.0.borrow_mut() {
            CheckedValue::Array(_, arr) => {
                let index = path[0];
                let rest = &path[1..];
                if let Some(inner) = arr.get_mut(index) {
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
                unreachable!()
            }
        }
        Ok(())
    }

    pub fn get_path(&self, path: &[usize]) -> Option<CheckedValueRef<F>> {
        if path.is_empty() {
            return Some(self.clone());
        }

        let value = self.0.borrow();
        match &*value {
            CheckedValue::Array(_, arr) => {
                let index = path[0];
                let rest = &path[1..];
                arr.get(index).and_then(|inner| inner.get_path(rest))
            }
            CheckedValue::Struct(_, map) => {
                let key = IdentId(path[0]);
                let rest = &path[1..];
                map.get(&key).and_then(|inner| inner.get_path(rest))
            }
            _ => None,
        }
    }
}
