use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use either::Either;
use indexmap::IndexMap;
use qed_ast::{ExprId, IdentId, NodeType};
pub use strum::{EnumIs, EnumTryAs};

use crate::{TypeId, BOOL_TYPE, FELT_TYPE};

#[derive(Clone, Debug, PartialEq, EnumIs, EnumTryAs)]
pub enum CheckedValueNode<F> {
    Felt(F),
    Bool(F),
    Array(TypeId, Vec<ExprId>),
    Struct(TypeId, IndexMap<IdentId, ExprId>),
    Type(TypeId),
}

#[derive(Clone, Debug, PartialEq, EnumIs, EnumTryAs)]
pub enum CheckedValue<F> {
    Felt(F),
    Bool(F),
    Array(TypeId, Vec<CheckedValue<F>>),
    Struct(TypeId, IndexMap<IdentId, CheckedValue<F>>),
    Type(TypeId),
}

impl<F: Clone + From<u32>> CheckedValue<F> {
    pub fn to_array(&self) -> Vec<F> {
        match self {
            CheckedValue::Felt(f) => vec![f.clone()],
            CheckedValue::Bool(b) => vec![b.clone()],
            CheckedValue::Array(type_id, values) => {
                let mut result = Vec::new();
                for value in values {
                    result.extend(value.to_array());
                }
                result
            }
            CheckedValue::Struct(type_id, fields) => {
                let mut result = Vec::new();
                for (_, value) in fields {
                    result.extend(value.to_array());
                }
                result
            }
            CheckedValue::Type(type_id) => {
                unreachable!()
            }
        }
    }

    pub fn from_array(self, arr: &[F]) -> (Self, &[F]) {
        todo!()
        // match &self {
        //     CheckedValue::Felt(_) => (CheckedValue::Felt(arr[0].clone()), &[]),
        //     CheckedValue::Bool(_) => (CheckedValue::Bool(arr[0].clone()), &[]),
        //     CheckedValue::Array(, v) => {
        //         let mut values = Vec::with_capacity(template_values.len());
        //         let mut remaining = arr;
        //
        //         // 使用模板数组的长度和元素类型
        //         for template_value in template_values {
        //             let (value, new_remaining) = Self::from_array(remaining, template_value)?;
        //             values.push(value);
        //             remaining = new_remaining;
        //         }
        //
        //         Ok((CheckedValue::Array(*type_id, values), remaining))
        //     }
        //     CheckedValue::Struct(type_id, template_fields) => {
        //         let mut fields = IndexMap::new();
        //         let mut remaining = arr;
        //
        //         // 使用模板结构体的字段顺序和类型
        //         for (field_name, template_value) in template_fields {
        //             let (value, new_remaining) = Self::from_array(remaining, template_value)?;
        //             fields.insert(*field_name, value);
        //             remaining = new_remaining;
        //         }
        //
        //         Ok((CheckedValue::Struct(*type_id, fields), remaining))
        //     }
        //     CheckedValue::Type(_) => {
        //         unreachable!("Type cannot be serialized")
        //     }
        // }
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
