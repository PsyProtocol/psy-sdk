use std::cell::RefCell;
use std::convert::AsMut;
use std::convert::AsRef;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;
use qed_ast::IdentId;
use qed_ast::Visibility;
use qed_utils::impl_ref;
use qedlang_core::dpn::ops::context_trait::{ContextFelt, DPNContext, ToFelts};

use crate::CheckedValue;
use crate::CheckedValueNode;
use crate::CheckedValueRef;
use crate::SymbolTable;
use crate::STD_PRELUDE_SCOPE_ID;
use crate::{
    CheckedArrayNode, CheckedEnumNode, CheckedFunctionNode, CheckedImplNode, CheckedStructNode,
    CheckedTraitNode, ScopeId,
};
use qed_common::define_arena_id;
use strum::EnumTryAs;

define_arena_id!(TypeId);

impl Display for TypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub const UNKOWN_TYPE: TypeId = TypeId(0);
pub const VOID_TYPE: TypeId = TypeId(1);
pub const BOOL_TYPE: TypeId = TypeId(2);
pub const FELT_TYPE: TypeId = TypeId(3);

pub const TYPE_MAPPING: &[(IdentId, Type)] = &[
    (IdentId::TYPE_UNKNOWN, Type::Unknown),
    (IdentId::TYPE_VOID, Type::VOID),
    (
        IdentId::TYPE_BOOL,
        Type::Bool(CheckedBoolNode {
            implementations: vec![],
        }),
    ),
    (
        IdentId::TYPE_FELT,
        Type::Felt(CheckedFeltNode {
            implementations: vec![],
        }),
    ),
];

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedBoolNode {
    pub implementations: Vec<TypeId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedFeltNode {
    pub implementations: Vec<TypeId>,
}

#[derive(Debug, Clone, PartialEq, EnumAsInner, EnumTryAs)]
pub enum Type {
    Unknown,
    VOID,
    Felt(CheckedFeltNode),
    Bool(CheckedBoolNode),
    Array(CheckedArrayNode),
    Struct(CheckedStructNode),
    Enum(CheckedEnumNode),
    Function(CheckedFunctionNode),
    Impl(CheckedImplNode),
    Trait(CheckedTraitNode),
    TypeVariable(IdentId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeKey {
    pub id: IdentId,
    pub generic_parameters: Vec<TypeId>,
    pub consts: Vec<usize>,
}

impl TypeKey {
    pub fn new(id: IdentId, generic_parameters: Vec<TypeId>, consts: Vec<usize>) -> Self {
        Self {
            id,
            generic_parameters,
            consts,
        }
    }
}

impl_ref!(Type,
    Array => CheckedArrayNode,
    Struct => CheckedStructNode,
    Enum => CheckedEnumNode,
    Function => CheckedFunctionNode,
    Impl => CheckedImplNode,
    Trait => CheckedTraitNode
);

impl From<IdentId> for TypeKey {
    fn from(value: IdentId) -> Self {
        TypeKey::new(value, vec![], vec![])
    }
}

impl Type {
    pub fn key(&self) -> TypeKey {
        let (id, generic_parameters, consts) = match self {
            Type::Unknown => (IdentId::TYPE_UNKNOWN, vec![], vec![]),
            Type::VOID => (IdentId::TYPE_VOID, vec![], vec![]),
            Type::Felt(_) => (IdentId::TYPE_FELT, vec![], vec![]),
            Type::Bool(_) => (IdentId::TYPE_BOOL, vec![], vec![]),
            Type::Array(CheckedArrayNode {
                inner_ty,
                size,
                scope_id,
            }) => (
                IdentId::TYPE_ARRAY,
                vec![inner_ty.clone()],
                vec![size.clone()],
            ),
            Type::Struct(CheckedStructNode {
                name,
                generic_parameters,
                ..
            }) => (name.clone(), generic_parameters.clone(), vec![]),
            Type::Enum(CheckedEnumNode {
                name,
                generic_parameters,
                ..
            }) => (name.clone(), generic_parameters.clone(), vec![]),
            Type::Function(CheckedFunctionNode {
                name,
                generic_parameters,
                ..
            }) => (name.clone(), generic_parameters.clone(), vec![]),
            Type::Impl(CheckedImplNode {
                generic_parameters,
                ty,
                ..
            }) => (ty.clone(), generic_parameters.clone(), vec![]),
            Type::Trait(CheckedTraitNode {
                generic_parameters,
                name,
                ..
            }) => (name.clone(), generic_parameters.clone(), vec![]),
            Type::TypeVariable(id) => panic!("Type::id called on TypeVariable type"),
        };
        TypeKey::new(id, generic_parameters, consts)
    }

    pub fn scope_id(&self) -> ScopeId {
        match self {
            Type::Array(CheckedArrayNode { scope_id, .. }) => *scope_id,
            Type::Struct(CheckedStructNode { scope_id, .. }) => *scope_id,
            Type::Enum(CheckedEnumNode { scope_id, .. }) => *scope_id,
            Type::Function(CheckedFunctionNode { scope_id, .. }) => *scope_id,
            Type::Impl(CheckedImplNode { scope_id, .. }) => *scope_id,
            Type::Trait(CheckedTraitNode { scope_id, .. }) => *scope_id,
            Type::Felt(_) => ScopeId::prelude(),
            Type::Bool(_) => ScopeId::prelude(),
            _ => panic!("Type::scope_id called on non-composite type"),
        }
    }

    pub fn add_implementation(&mut self, trait_type_id: TypeId) {
        match self {
            Type::Struct(CheckedStructNode {
                ref mut implementations,
                ..
            }) => {
                implementations.push(trait_type_id);
            }
            Type::Enum(CheckedEnumNode {
                ref mut implementations,
                ..
            }) => {
                implementations.push(trait_type_id);
            }
            Type::Felt(CheckedFeltNode {
                ref mut implementations,
                ..
            }) => {
                implementations.push(trait_type_id);
            }
            Type::Bool(CheckedBoolNode {
                ref mut implementations,
                ..
            }) => {
                implementations.push(trait_type_id);
            }
            _ => panic!("Type::add_implementation called on non-composite type"),
        }
    }

    pub fn implementations(&self) -> &Vec<TypeId> {
        match self {
            Type::Struct(CheckedStructNode {
                implementations, ..
            }) => implementations,
            Type::Enum(CheckedEnumNode {
                implementations, ..
            }) => implementations,
            Type::Felt(CheckedFeltNode {
                implementations, ..
            }) => implementations,
            Type::Bool(CheckedBoolNode {
                implementations, ..
            }) => implementations,
            _ => panic!("Type::implementations called on non-composite type"),
        }
    }

    pub fn visibility(&self) -> Visibility {
        match self {
            Type::Struct(CheckedStructNode { visibility, .. }) => *visibility,
            Type::Enum(CheckedEnumNode { visibility, .. }) => *visibility,
            Type::Function(CheckedFunctionNode { visibility, .. }) => *visibility,
            Type::Trait(CheckedTraitNode { visibility, .. }) => *visibility,
            _ => Visibility::Public,
        }
    }

    pub fn to_value<F: ContextFelt + From<u32>, C: DPNContext<F>>(
        &self,
        symbols: &SymbolTable<F>,
        ctx: &mut C,
    ) -> CheckedValue<F> {
        match self {
            Type::Felt(f) => CheckedValue::Felt(ctx.add_input()),
            Type::Bool(b) => CheckedValue::Bool(ctx.add_input()),
            Type::Array(a) => {
                let mut result = Vec::new();
                let inner_ty = symbols[a.inner_ty].clone();
                for value in 0..a.size {
                    result.push(CheckedValueRef::new_rc(inner_ty.to_value(symbols, ctx)));
                }
                let type_id = symbols
                    .get_type_id(Some(ScopeId::prelude()), self.key())
                    .unwrap();
                CheckedValue::Array(type_id, result)
            }
            Type::Struct(s) => {
                let mut result = IndexMap::new();
                for (field_name, (field_type, _)) in &s.fields {
                    let field_type = symbols[field_type.clone()].clone();
                    result.insert(
                        field_name.clone(),
                        CheckedValueRef::new_rc(field_type.to_value(symbols, ctx)),
                    );
                }
                let type_id = symbols.get_type_id(Some(s.scope_id), self.key()).unwrap();
                CheckedValue::Struct(type_id, result)
            }
            _ => unreachable!(),
        }
    }
}
