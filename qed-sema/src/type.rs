use std::convert::AsMut;
use std::convert::AsRef;

use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;
use qed_ast::IdentId;
use qed_ast::Visibility;
use qed_utils::impl_ref;
use qedlang_core::dpn::ops::context_trait::{ContextFelt, DPNContext};

use crate::CheckedConstNode;
use crate::CheckedFunctionSignature;
use crate::CheckedValue;
use crate::CheckedValueRef;
use crate::SymbolTable;
use crate::{
    CheckedArrayNode, CheckedEnumNode, CheckedFunctionNode, CheckedStructNode, CheckedTraitNode,
    ScopeId,
};
use qed_common::define_arena_id;
use strum::EnumTryAs;

define_arena_id!(TypeId);

pub const UNKOWN_TYPE: TypeId = TypeId(0);
pub const VOID_TYPE: TypeId = TypeId(1);
pub const BOOL_TYPE: TypeId = TypeId(2);
pub const FELT_TYPE: TypeId = TypeId(3);
pub const HASH_TYPE: TypeId = TypeId(4);

use once_cell::sync::Lazy;

pub static TYPE_MAPPING: Lazy<Vec<(IdentId, Type)>> = Lazy::new(|| {
    vec![
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
        (
            IdentId::TYPE_HASH,
            Type::Array(CheckedArrayNode {
                inner_ty: FELT_TYPE,
                size: 4,
            }),
        ),
    ]
});

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
    FunctionSignature(CheckedFunctionSignature),
    Trait(CheckedTraitNode),
    Const(CheckedConstNode),
    TypeVariable(IdentId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeKey {
    pub name: IdentId,
    pub generic_parameters: Vec<TypeId>,
    pub parameters: Vec<(bool, TypeId)>,
    pub return_type: Option<TypeId>,
    pub consts: Vec<usize>,
}

impl TypeKey {
    pub fn new(
        name: IdentId,
        generic_parameters: Vec<TypeId>,
        parameters: Vec<(bool, TypeId)>,
        return_type: Option<TypeId>,
        consts: Vec<usize>,
    ) -> Self {
        Self {
            name,
            generic_parameters,
            parameters,
            return_type,
            consts,
        }
    }
}

impl_ref!(Type,
    Array => CheckedArrayNode,
    Struct => CheckedStructNode,
    Enum => CheckedEnumNode,
    Function => CheckedFunctionNode,
    Trait => CheckedTraitNode
);

impl From<IdentId> for TypeKey {
    fn from(value: IdentId) -> Self {
        TypeKey::new(value, vec![], vec![], None, vec![])
    }
}

impl Type {
    pub fn key(&self) -> TypeKey {
        let (name, generic_parameters, parameters, return_type, consts) = match self {
            Type::Unknown => (IdentId::TYPE_UNKNOWN, vec![], vec![], None, vec![]),
            Type::VOID => (IdentId::TYPE_VOID, vec![], vec![], None, vec![]),
            Type::Felt(_) => (IdentId::TYPE_FELT, vec![], vec![], None, vec![]),
            Type::Bool(_) => (IdentId::TYPE_BOOL, vec![], vec![], None, vec![]),
            Type::Array(CheckedArrayNode { inner_ty, size, .. }) => (
                IdentId::TYPE_ARRAY,
                vec![inner_ty.clone()],
                vec![],
                None,
                vec![size.clone()],
            ),
            Type::Struct(CheckedStructNode {
                name,
                generic_parameters,
                ..
            }) => (
                name.clone(),
                generic_parameters.clone(),
                vec![],
                None,
                vec![],
            ),
            Type::Enum(CheckedEnumNode {
                name,
                generic_parameters,
                ..
            }) => (
                name.clone(),
                generic_parameters.clone(),
                vec![],
                None,
                vec![],
            ),
            Type::Function(CheckedFunctionNode {
                name,
                generic_parameters,
                ..
            }) => (
                name.clone(),
                generic_parameters.clone(),
                vec![],
                None,
                vec![],
            ),
            Type::FunctionSignature(CheckedFunctionSignature {
                parameters,
                return_type,
            }) => (
                IdentId::FN_SIG,
                vec![],
                parameters
                    .iter()
                    .map(|(mutable, ty)| (mutable.clone(), ty.clone()))
                    .collect(),
                match return_type {
                    &VOID_TYPE => None,
                    ty => Some(ty.clone()),
                },
                vec![],
            ),
            Type::Trait(CheckedTraitNode {
                name,
                generic_parameters,
                ..
            }) => (
                name.clone(),
                generic_parameters.clone(),
                vec![],
                None,
                vec![],
            ),
            Type::Const(CheckedConstNode { name, .. }) => {
                (name.clone(), vec![], vec![], None, vec![])
            }
            _ => panic!("Type::key called on TypeVariable type"),
        };
        TypeKey::new(name, generic_parameters, parameters, return_type, consts)
    }

    pub fn scope_id(&self) -> ScopeId {
        match self {
            Type::Array(_) => ScopeId::primitive(),
            Type::Struct(CheckedStructNode { scope_id, .. }) => *scope_id,
            Type::Enum(CheckedEnumNode { scope_id, .. }) => *scope_id,
            Type::Function(CheckedFunctionNode { scope_id, .. }) => *scope_id,
            Type::Trait(CheckedTraitNode { scope_id, .. }) => *scope_id,
            Type::Const(CheckedConstNode { scope_id, .. }) => *scope_id,
            Type::Felt(_) => ScopeId::primitive(),
            Type::Bool(_) => ScopeId::primitive(),
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
            Type::Const(CheckedConstNode { visibility, .. }) => *visibility,
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
                for _ in 0..a.size {
                    result.push(CheckedValueRef::new_rc(inner_ty.to_value(symbols, ctx)));
                }
                let type_id = symbols
                    .get_type_id(Some(ScopeId::primitive()), self.key())
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
