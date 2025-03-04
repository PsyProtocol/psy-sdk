use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;
use qed_ast::{ExprId, IdentId, NodeInfo, NodeType};
use qedlang_core::dpn::ops::context_trait::{ContextFelt, DPNContext, DPNContextArray, ToFelts};

use crate::{Result, TypeId, BOOL_TYPE, FELT_TYPE, U32_TYPE, VOID_TYPE};

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum CheckedValueNode<F> {
    Felt(F),
    Bool(F),
    U32(F),
    Array(TypeId, Vec<ExprId>),
    Tuple(TypeId, Vec<(TypeId, ExprId)>),
    Struct(TypeId, IndexMap<IdentId, ExprId>),
    Type(TypeId),
}

impl<F> NodeInfo for CheckedValueNode<F> {
    fn node_type(&self) -> NodeType {
        NodeType::ValueExpr
    }
}

#[derive(Debug, EnumAsInner)]
pub enum CheckedValue<F: Clone + From<u32> + ContextFelt> {
    Felt(F),
    Bool(F),
    U32(F),
    Array(TypeId, Vec<CheckedValueRef<F>>),
    Struct(TypeId, IndexMap<IdentId, CheckedValueRef<F>>),
    Tuple {
        type_id: TypeId,
        elements: Vec<(TypeId, CheckedValueRef<F>)>,
    },
    Type(TypeId),
    Stash(Vec<F>),
}

#[derive(Debug)]
pub struct CheckedValueRef<F: Clone + From<u32> + ContextFelt>(Rc<RefCell<CheckedValue<F>>>);

impl<F: Clone + From<u32> + ContextFelt> Clone for CheckedValueRef<F> {
    fn clone(&self) -> Self {
        match &*self.0.borrow() {
            CheckedValue::Felt(f) => {
                CheckedValueRef(Rc::new(RefCell::new(CheckedValue::Felt(f.clone()))))
            }
            CheckedValue::Bool(b) => {
                CheckedValueRef(Rc::new(RefCell::new(CheckedValue::Bool(b.clone()))))
            }
            CheckedValue::U32(u) => {
                CheckedValueRef(Rc::new(RefCell::new(CheckedValue::U32(u.clone()))))
            }
            CheckedValue::Array(_type_id, _values) => CheckedValueRef(self.0.clone()),
            CheckedValue::Tuple { type_id, elements } => {
                CheckedValueRef(Rc::new(RefCell::new(CheckedValue::Tuple {
                    type_id: type_id.clone(),
                    elements: elements
                        .iter()
                        .map(|(t, v)| (t.clone(), v.clone()))
                        .collect(),
                })))
            }
            CheckedValue::Struct(_type_id, _fields) => CheckedValueRef(self.0.clone()),
            CheckedValue::Type(type_id) => {
                CheckedValueRef(Rc::new(RefCell::new(CheckedValue::Type(type_id.clone()))))
            }
            CheckedValue::Stash(data) => {
                CheckedValueRef(Rc::new(RefCell::new(CheckedValue::Stash(data.clone()))))
            }
        }
    }
}

impl<F: Clone + From<u32> + ContextFelt> PartialEq for CheckedValueRef<F> {
    fn eq(&self, other: &Self) -> bool {
        match (&*self.0.borrow(), &*other.0.borrow()) {
            (CheckedValue::Felt(f1), CheckedValue::Felt(f2)) => f1 == f2,
            (CheckedValue::Bool(b1), CheckedValue::Bool(b2)) => b1 == b2,
            (CheckedValue::U32(u1), CheckedValue::U32(u2)) => u1 == u2,
            (CheckedValue::Array(_, _a1), CheckedValue::Array(_, _a2)) => {
                std::ptr::eq(Rc::as_ptr(&self.as_rc()), Rc::as_ptr(&other.as_rc()))
            }
            (CheckedValue::Struct(_, _s1), CheckedValue::Struct(_, _s2)) => {
                std::ptr::eq(Rc::as_ptr(&self.as_rc()), Rc::as_ptr(&other.as_rc()))
            }
            (CheckedValue::Type(t1), CheckedValue::Type(t2)) => t1 == t2,
            (CheckedValue::Stash(d1), CheckedValue::Stash(d2)) => d1 == d2,
            _ => false,
        }
    }
}

impl<F: Clone + From<u32> + ContextFelt> ToFelts<F> for CheckedValueRef<F> {
    fn to_felts(&self) -> Vec<F> {
        match &*self.0.borrow() {
            CheckedValue::Felt(f) => vec![f.clone()],
            CheckedValue::Bool(b) => vec![b.clone()],
            CheckedValue::U32(u) => vec![u.clone()],
            CheckedValue::Array(_type_id, values) => {
                let mut result = Vec::new();
                for value in values {
                    result.extend(value.to_felts());
                }
                result
            }
            CheckedValue::Tuple { elements, .. } => {
                elements.iter().flat_map(|(_, v)| v.to_felts()).collect()
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
            CheckedValue::Stash(data) => data.clone(),
        }
    }

    fn from_felts(felts: &[F]) -> Self {
        if felts.len() == 1 {
            Self::from_felt(felts[0].clone())
        } else {
            Self::new_rc(CheckedValue::Stash(felts.to_vec()))
        }
    }
}

impl<F: Clone + From<u32> + ContextFelt> CheckedValueRef<F> {
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

    pub fn from_u32(value: F) -> Self {
        Self::new_rc(CheckedValue::U32(value))
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

    pub fn is_u32(&self) -> bool {
        self.0.borrow().is_u32()
    }

    pub fn is_struct(&self) -> bool {
        self.0.borrow().is_struct()
    }

    pub fn is_array(&self) -> bool {
        self.0.borrow().is_array()
    }
    pub fn is_tuple(&self) -> bool {
        self.0.borrow().is_tuple()
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

    pub fn to_u32(&self) -> F {
        match &*self.0.borrow() {
            CheckedValue::U32(f) => f.clone(),
            _ => panic!("Expected u32 value"),
        }
    }

    pub fn to_value(&self) -> F {
        match &*self.0.borrow() {
            CheckedValue::Felt(f) => f.clone(),
            CheckedValue::U32(u) => u.clone(),
            CheckedValue::Bool(b) => b.clone(),
            _ => panic!("Expected felt/u32/bool value"),
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
            CheckedValue::U32(_) => U32_TYPE,
            CheckedValue::Array(type_id, _) => type_id.clone(),
            CheckedValue::Struct(type_id, _) => type_id.clone(),
            CheckedValue::Type(type_id) => type_id.clone(),
            CheckedValue::Tuple { type_id, .. } => type_id.clone(),
            CheckedValue::Stash(_) => unreachable!(),
        }
    }

    pub fn felt_size(&self) -> usize {
        match &*self.0.borrow() {
            CheckedValue::Felt(_) => 1,
            CheckedValue::Bool(_) => 1,
            CheckedValue::U32(_) => 1,
            CheckedValue::Array(_, values) => {
                let mut result = 0;
                for value in values {
                    result += value.felt_size();
                }
                result
            }
            CheckedValue::Struct(_, fields) => {
                let mut result = 0;
                for (_, value) in fields {
                    result += value.felt_size();
                }
                result
            }
            CheckedValue::Type(_) => {
                unreachable!()
            }
            CheckedValue::Tuple { elements, .. } => {
                let mut result = 0;
                for (_, elem) in elements {
                    result += elem.felt_size();
                }
                result
            }
            CheckedValue::Stash(data) => data.len(),
        }
    }

    pub fn set_path<C>(&mut self, ctx: &mut C, path: &[IndexPath<F>], value: Self) -> Result<()>
    where
        F: Clone + From<u32> + ContextFelt,
        C: DPNContext<F>,
    {
        if path.is_empty() {
            *self = value.clone();
            return Ok(());
        }

        match &mut *self.0.borrow_mut() {
            CheckedValue::Array(_, arr) => {
                let index = path[0].as_felt().unwrap();
                let rest = &path[1..];
                let res = arr.q_get(ctx, *index);
                Self::convert(arr[0].clone(), res).set_path(ctx, rest, value)?;
            }
            CheckedValue::Tuple { elements, .. } => {
                let index = path[0].as_normal().unwrap();
                let rest = &path[1..];
                if let Some((_, inner)) = elements.get_mut(*index) {
                    inner.set_path(ctx, rest, value)?;
                }
            }
            CheckedValue::Struct(_, map) => {
                let key = IdentId(*path[0].as_normal().unwrap());
                let rest = &path[1..];
                if let Some(inner) = map.get_mut(&key) {
                    inner.set_path(ctx, rest, value)?;
                }
            }
            _ => {
                unreachable!()
            }
        }
        Ok(())
    }

    pub fn get_path<C>(&self, ctx: &mut C, path: &[IndexPath<F>]) -> Option<CheckedValueRef<F>>
    where
        F: Clone + From<u32> + ContextFelt,
        C: DPNContext<F>,
    {
        if path.is_empty() {
            return Some(self.clone());
        }

        let value = self.0.borrow();
        match &*value {
            CheckedValue::Array(_, arr) => {
                let index = path[0].clone().into_felt().unwrap();
                let rest = &path[1..];
                let res = arr.q_get(ctx, index);
                Self::convert(arr[0].clone(), res).get_path(ctx, rest)
            }
            CheckedValue::Tuple { elements, .. } => {
                let index = path[0].clone().into_normal().unwrap();
                let rest = &path[1..];
                elements
                    .get(index)
                    .and_then(|(_, inner)| inner.get_path(ctx, rest))
            }
            CheckedValue::Struct(_, map) => {
                let key = IdentId(path[0].clone().into_normal().unwrap());
                let rest = &path[1..];
                map.get(&key).and_then(|inner| inner.get_path(ctx, rest))
            }
            _ => None,
        }
    }

    pub fn convert(value_type: CheckedValueRef<F>, value: CheckedValueRef<F>) -> CheckedValueRef<F>
    where
        F: Clone + From<u32> + ContextFelt,
    {
        assert!(value_type.felt_size() == value.felt_size());
        let value_felts = value.to_felts();
        match &*value_type.0.borrow() {
            CheckedValue::Felt(_) => {
                assert!(value_felts.len() == 1);
                CheckedValueRef::new_rc(CheckedValue::Felt(value_felts[0].clone()))
            }
            CheckedValue::Bool(_) => {
                assert!(value_felts.len() == 1);
                CheckedValueRef::new_rc(CheckedValue::Bool(value_felts[0].clone()))
            }
            CheckedValue::Array(type_id, arr) => {
                assert!(value_felts.len() % arr.len() == 0);
                let arr_data = value_felts
                    .chunks(value_felts.len() / arr.len())
                    .zip(arr.iter())
                    .map(|(value, val_type)| {
                        Self::convert(
                            val_type.clone(),
                            CheckedValueRef::new_rc(CheckedValue::Stash(value.to_vec())),
                        )
                    })
                    .collect();
                CheckedValueRef::new_rc(CheckedValue::Array(type_id.clone(), arr_data))
            }
            CheckedValue::Tuple { type_id, elements } => {
                let mut index = 0;
                let mut tuple_elements = Vec::new();
                for (elem_type, elem_value) in elements.iter() {
                    let elem_size = elem_value.felt_size();
                    let elem_values = &value_felts[index..index + elem_size];
                    index += elem_size;
                    tuple_elements.push((
                        elem_type.clone(),
                        Self::convert(
                            elem_value.clone(),
                            CheckedValueRef::new_rc(CheckedValue::Stash(elem_values.to_vec())),
                        ),
                    ));
                }
                CheckedValueRef::new_rc(CheckedValue::Tuple {
                    type_id: type_id.clone(),
                    elements: tuple_elements,
                })
            }
            CheckedValue::Struct(type_id, fields) => {
                let mut index = 0;
                let mut fields_map = IndexMap::new();
                for (ident_id, field) in fields.iter() {
                    let field_values = &value_felts[index..index + field.felt_size()];
                    index += field.felt_size();
                    fields_map.insert(
                        ident_id.clone(),
                        Self::convert(
                            field.clone(),
                            CheckedValueRef::new_rc(CheckedValue::Stash(field_values.to_vec())),
                        ),
                    );
                }
                CheckedValueRef::new_rc(CheckedValue::Struct(type_id.clone(), fields_map))
            }
            _ => unimplemented!(),
        }
    }
}

#[derive(Clone, Debug, EnumAsInner)]
pub enum IndexPath<F: ContextFelt> {
    Normal(usize),
    Felt(F),
}
