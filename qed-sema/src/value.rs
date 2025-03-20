use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;
use qed_ast::{ExprId, IdentId, Location, NodeInfo, NodeType};
use qedlang_core::dpn::ops::{
    context_trait::{ContextFelt, DPNContext, ToFelts},
    op_types::DPNOpType,
};

use crate::{Result, TypeId, BOOL_TYPE, FELT_TYPE, U32_TYPE, VOID_TYPE};

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum CheckedValueNode<F> {
    Felt(F, Location),
    Bool(F, Location),
    U32(F, Location),
    Array(TypeId, Vec<ExprId>, Location),
    Tuple(TypeId, Vec<(TypeId, ExprId)>, Location),
    Struct(TypeId, IndexMap<IdentId, ExprId>, Location),
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
}

/// **NOTE**: please dont implement Deref for CheckedValueRef in case of wrong usage
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
        }
    }

    fn from_felts(_felts: &[F]) -> Self {
        todo!()
    }
}

impl<F: Clone + From<u32> + ContextFelt> CheckedValueRef<F> {
    pub fn new_rc(value: CheckedValue<F>) -> Self {
        Self(Rc::new(RefCell::new(value)))
    }

    pub fn as_rc(&self) -> Rc<RefCell<CheckedValue<F>>> {
        Rc::clone(&self.0)
    }

    pub fn as_type(&self) -> Option<TypeId> {
        match &*self.0.borrow() {
            CheckedValue::Type(type_id) => Some(type_id.clone()),
            _ => None,
        }
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
        }
    }

    pub fn set_path<C>(
        &mut self,
        ctx: &mut C,
        path: &[IndexPath<F>],
        index_condition: &mut Vec<F>,
        value: Self,
    ) -> Result<()>
    where
        F: Clone + From<u32> + ContextFelt,
        C: DPNContext<F>,
    {
        if path.is_empty() {
            if index_condition.is_empty() {
                *self = value.clone();
            } else {
                let combine_condition = index_condition
                    .iter()
                    .fold(ctx.op_true(), |acc, condition| {
                        ctx.op_bool_and(*condition, acc)
                    });
                *self = Self::select(ctx, &value, &self, &|ctx: &mut C, n: &F, o: &F| {
                    ctx.op_select(combine_condition, n.clone(), o.clone())
                });
            }
            return Ok(());
        }

        match &mut *self.0.borrow_mut() {
            CheckedValue::Array(_, arr) => {
                let index = path[0].as_felt().unwrap();
                let rest = &path[1..];
                let const_types = [
                    DPNOpType::Constant,
                    DPNOpType::ConstantTrue,
                    DPNOpType::ConstantFalse,
                    DPNOpType::ConstantU32,
                ];
                if const_types.contains(&ctx.get_op_type(*index)) {
                    let index = ctx.get_constant_value(*index) as usize;
                    if let Some(inner) = arr.get_mut(index) {
                        inner.set_path(ctx, rest, index_condition, value)?;
                    }
                } else {
                    let arr_size = ctx.op_const(arr.len() as u64);
                    let out_of_bounds = ctx.op_lt(*index, arr_size);
                    ctx.assert_true(out_of_bounds, "felt index out of bounds");
                    for i in 0..arr.len() {
                        let arr_index = ctx.op_const(i as u64);
                        let condition = ctx.op_eq(arr_index, *index);
                        index_condition.push(condition);
                        if let Some(inner) = arr.get_mut(i) {
                            inner.set_path(ctx, rest, index_condition, value.clone())?;
                        }
                        index_condition.pop();
                    }
                }
            }
            CheckedValue::Tuple { elements, .. } => {
                let index = path[0].as_normal().unwrap();
                let rest = &path[1..];
                if let Some((_, inner)) = elements.get_mut(*index) {
                    inner.set_path(ctx, rest, index_condition, value)?;
                }
            }
            CheckedValue::Struct(_, map) => {
                let key = IdentId(*path[0].as_normal().unwrap());
                let rest = &path[1..];
                if let Some(inner) = map.get_mut(&key) {
                    inner.set_path(ctx, rest, index_condition, value)?;
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
                let const_types = [
                    DPNOpType::Constant,
                    DPNOpType::ConstantTrue,
                    DPNOpType::ConstantFalse,
                    DPNOpType::ConstantU32,
                ];
                if const_types.contains(&ctx.get_op_type(index)) {
                    let index = ctx.get_constant_value(index) as usize;
                    arr.get(index).and_then(|inner| inner.get_path(ctx, rest))
                } else {
                    let arr_size = ctx.op_const(arr.len() as u64);
                    let out_of_bounds = ctx.op_lt(index, arr_size);
                    ctx.assert_true(out_of_bounds, "felt index out of bounds");

                    let mut result = arr[0].clone();
                    for i in 1..arr.len() {
                        let arr_index = ctx.op_const(i as u64);
                        let condition = ctx.op_eq(arr_index, index);
                        result =
                            Self::select(ctx, &arr[i], &result, &|ctx: &mut C, n: &F, o: &F| {
                                ctx.op_select(condition, n.clone(), o.clone())
                            });
                    }
                    result.get_path(ctx, rest)
                }
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

    pub fn select<C>(
        ctx: &mut C,
        new_value: &CheckedValueRef<F>,
        old_value: &CheckedValueRef<F>,
        select_fn: &impl Fn(&mut C, &F, &F) -> F,
    ) -> CheckedValueRef<F>
    where
        F: Clone + From<u32> + ContextFelt,
        C: DPNContext<F>,
    {
        if old_value == new_value {
            return old_value.clone();
        }
        match (&*old_value.borrow(), &*new_value.borrow()) {
            (CheckedValue::Felt(o), CheckedValue::Felt(n)) => {
                CheckedValueRef::new_rc(CheckedValue::Felt(select_fn(ctx, n, o)))
            }
            (CheckedValue::Bool(o), CheckedValue::Bool(n)) => {
                CheckedValueRef::new_rc(CheckedValue::Bool(select_fn(ctx, n, o)))
            }
            (CheckedValue::U32(o), CheckedValue::U32(n)) => {
                CheckedValueRef::new_rc(CheckedValue::U32(select_fn(ctx, n, o)))
            }
            (CheckedValue::Array(lhs_type_id, o), CheckedValue::Array(_, n)) => {
                let mut arr_data = vec![];
                for (old_value, new_value) in o.iter().zip(n.iter()) {
                    arr_data.push(Self::select(ctx, new_value, old_value, select_fn));
                }
                CheckedValueRef::new_rc(CheckedValue::Array(lhs_type_id.clone(), arr_data))
            }
            (CheckedValue::Struct(lhs_type_id, o), CheckedValue::Struct(_, n)) => {
                let mut struct_map = IndexMap::new();
                for ((old_field_name, old_field_value), (new_field_name, new_field_value)) in
                    o.iter().zip(n.iter())
                {
                    assert_eq!(old_field_name, new_field_name);
                    struct_map.insert(
                        old_field_name.clone(),
                        Self::select(ctx, new_field_value, old_field_value, select_fn),
                    );
                }
                CheckedValueRef::new_rc(CheckedValue::Struct(lhs_type_id.clone(), struct_map))
            }
            (
                CheckedValue::Tuple {
                    type_id: lhs_tid,
                    elements: old_elements,
                },
                CheckedValue::Tuple {
                    elements: new_elements,
                    ..
                },
            ) => {
                assert_eq!(
                    old_elements.len(),
                    new_elements.len(),
                    "Tuple size mismatch"
                );

                let mut tuple_elements = vec![];
                for ((old_type_id, old_value), (_, new_value)) in
                    old_elements.iter().zip(new_elements.iter())
                {
                    tuple_elements.push((
                        old_type_id.clone(),
                        Self::select(ctx, new_value, old_value, select_fn),
                    ));
                }
                CheckedValueRef::new_rc(CheckedValue::Tuple {
                    type_id: lhs_tid.clone(),
                    elements: tuple_elements,
                })
            }
            _ => {
                unreachable!()
            }
        }
    }
}

#[derive(Clone, Debug, EnumAsInner)]
pub enum IndexPath<F: ContextFelt> {
    Normal(usize),
    Felt(F),
}
