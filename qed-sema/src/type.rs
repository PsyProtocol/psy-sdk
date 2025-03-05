use std::convert::AsMut;
use std::convert::AsRef;

use enum_as_inner::EnumAsInner;
use qed_ast::TypeQualifier;
use qed_ast::Visibility;
use qed_ast::{ExprId, IdentId};
use qed_utils::impl_ref;

use crate::CheckedConstNode;
use crate::CheckedFunctionSignature;
use crate::CheckedLambdaFunctionNode;
use crate::ConstId;
use crate::{
    CheckedArrayNode, CheckedEnumNode, CheckedFunctionNode, CheckedStructNode, CheckedTraitNode,
    ScopeId,
};
use qed_common::define_arena_id;

define_arena_id!(TypeId);

pub const UNKOWN_TYPE: TypeId = TypeId(0);
pub const VOID_TYPE: TypeId = TypeId(1);
pub const BOOL_TYPE: TypeId = TypeId(2);
pub const FELT_TYPE: TypeId = TypeId(3);
pub const U32_TYPE: TypeId = TypeId(4);
pub const HASH_TYPE: TypeId = TypeId(9);

use once_cell::sync::Lazy;

pub static PRIMITIVE_TYPES: Lazy<Vec<Type>> = Lazy::new(|| {
    vec![
        Type::Unknown,
        Type::VOID,
        Type::Bool(CheckedBoolNode {
            implementations: vec![],
        }),
        Type::Felt(CheckedFeltNode {
            implementations: vec![],
        }),
        Type::U32(CheckedU32Node {
            implementations: vec![],
        }),
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

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedU32Node {
    pub implementations: Vec<TypeId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedTypeVariableNode {
    pub constraints: Vec<TypeId>,
    pub scope_id: ScopeId,
}

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum Type {
    Unknown,
    VOID,
    Felt(CheckedFeltNode),
    Bool(CheckedBoolNode),
    U32(CheckedU32Node),
    Array(CheckedArrayNode),
    Struct(CheckedStructNode),
    Enum(CheckedEnumNode),
    Function(CheckedFunctionNode),
    Trait(CheckedTraitNode),
    Const(CheckedConstNode),
    LambdaFunction(CheckedLambdaFunctionNode),
    FunctionSignature(CheckedFunctionSignature),
    TypeVariable(CheckedTypeVariableNode),
    Tuple(Vec<TypeId>),

    GenericInstance(TypeId, Vec<TypeId>),
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum TypeKind {
    Unknown,
    VOID,

    Felt,
    Bool,
    U32,
    Array,
    Struct,
    Enum,
    Tuple,
    Function,
    Trait,
    Const,

    LambdaFunction,
    FunctionSignature,
    TypeVariable,

    GenericInstance,
}

#[derive(Debug, Clone, Eq)]
pub struct TypeKey {
    pub name: Option<IdentId>,
    pub underlying_type_id: Option<TypeId>,
    pub generic_parameters: Vec<TypeId>,
    pub consts: Vec<usize>,

    pub parameters: Vec<TypeId>,
    pub return_type: Option<TypeId>,

    pub visibility: Visibility,
}

impl PartialEq for TypeKey {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.underlying_type_id == other.underlying_type_id
            && self.generic_parameters == other.generic_parameters
            && self.parameters == other.parameters
            && self.return_type == other.return_type
            && self.consts == other.consts
    }
}

impl std::hash::Hash for TypeKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.underlying_type_id.hash(state);
        self.generic_parameters.hash(state);
        self.parameters.hash(state);
        self.return_type.hash(state);
        self.consts.hash(state);
    }
}

impl TypeKey {
    pub fn new(
        name: Option<IdentId>,
        underlying_type_id: Option<TypeId>,
        generic_parameters: Vec<TypeId>,
        consts: Vec<usize>,
        parameters: Vec<TypeId>,
        return_type: Option<TypeId>,
    ) -> Self {
        Self {
            name,
            underlying_type_id,
            generic_parameters,
            consts,
            parameters,
            return_type,
            visibility: Visibility::Public,
        }
    }

    pub fn from_const_id(const_id: ConstId) -> Self {
        Self {
            name: None,
            underlying_type_id: None,
            generic_parameters: vec![],
            consts: vec![const_id.into()],
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
        }
    }
}

impl_ref!(Type,
    Array => CheckedArrayNode,
    Struct => CheckedStructNode,
    Enum => CheckedEnumNode,
    Function => CheckedFunctionNode,
    Trait => CheckedTraitNode,
    Const => CheckedConstNode,

    LambdaFunction => CheckedLambdaFunctionNode
);

impl From<IdentId> for TypeKey {
    fn from(value: IdentId) -> Self {
        TypeKey::new(Some(value), None, vec![], vec![], vec![], None)
    }
}

impl Type {
    pub fn key(&self) -> TypeKey {
        let (name, underlying_type_id, generic_parameters, consts, parameters, return_type) =
            match self {
                Type::Unknown => (
                    Some(IdentId::TYPE_UNKNOWN),
                    None,
                    vec![],
                    vec![],
                    vec![],
                    None,
                ),
                Type::VOID => (Some(IdentId::TYPE_VOID), None, vec![], vec![], vec![], None),
                Type::Felt(_) => (Some(IdentId::TYPE_FELT), None, vec![], vec![], vec![], None),
                Type::Bool(_) => (Some(IdentId::TYPE_BOOL), None, vec![], vec![], vec![], None),
                Type::U32(_) => (Some(IdentId::TYPE_U32), None, vec![], vec![], vec![], None),
                Type::Array(CheckedArrayNode { inner_ty, size, .. }) => (
                    Some(IdentId::TYPE_ARRAY),
                    None,
                    vec![inner_ty.clone(), size.clone()],
                    vec![],
                    vec![],
                    None,
                ),
                Type::Struct(CheckedStructNode {
                    name,
                    generic_parameters,
                    ..
                }) => (
                    Some(name.clone()),
                    None,
                    generic_parameters.clone(),
                    vec![],
                    vec![],
                    None,
                ),
                Type::Enum(CheckedEnumNode {
                    name,
                    generic_parameters,
                    ..
                }) => (
                    Some(name.clone()),
                    None,
                    generic_parameters.clone(),
                    vec![],
                    vec![],
                    None,
                ),
                Type::Tuple(elements) => (
                    Some(IdentId::TYPE_TUPLE),
                    None,
                    vec![],
                    vec![],
                    elements.clone(),
                    None,
                ),
                Type::Function(CheckedFunctionNode {
                    name,
                    generic_parameters,
                    ..
                }) => (
                    Some(name.clone()),
                    None,
                    generic_parameters.clone(),
                    vec![],
                    vec![],
                    None,
                ),
                Type::LambdaFunction(CheckedLambdaFunctionNode {
                    name,
                    parameters,
                    return_type,
                    ..
                }) => (
                    Some(name.clone()),
                    None,
                    vec![],
                    vec![],
                    parameters.iter().map(|(_, _, ty)| ty.clone()).collect(),
                    Some(return_type.clone()),
                ),
                Type::FunctionSignature(CheckedFunctionSignature {
                    parameters,
                    return_type,
                }) => (
                    None,
                    None,
                    vec![],
                    vec![],
                    parameters.clone(),
                    match return_type {
                        &VOID_TYPE => None,
                        ty => Some(ty.clone()),
                    },
                ),
                Type::Trait(CheckedTraitNode {
                    name,
                    generic_parameters,
                    ..
                }) => (
                    Some(name.clone()),
                    None,
                    generic_parameters.clone(),
                    vec![],
                    vec![],
                    None,
                ),
                Type::Const(CheckedConstNode { name, .. }) => {
                    (name.clone(), None, vec![], vec![], vec![], None)
                }
                Type::GenericInstance(underlying_type_id, generic_parameters) => (
                    None,
                    Some(underlying_type_id.clone()),
                    generic_parameters.clone(),
                    vec![],
                    vec![],
                    None,
                ),
                _ => panic!("Type::key called on TypeVariable type"),
            };
        TypeKey::new(
            name,
            underlying_type_id,
            generic_parameters,
            consts,
            parameters,
            return_type,
        )
    }

    pub fn scope_id(&self) -> ScopeId {
        match self {
            Type::Array(CheckedArrayNode { scope_id, .. }) => *scope_id,
            Type::Tuple(_) => ScopeId::primitive(),
            Type::Struct(CheckedStructNode { scope_id, .. }) => *scope_id,
            Type::Enum(CheckedEnumNode { scope_id, .. }) => *scope_id,
            Type::Function(CheckedFunctionNode { scope_id, .. }) => *scope_id,
            Type::Trait(CheckedTraitNode { scope_id, .. }) => *scope_id,
            Type::Const(CheckedConstNode { scope_id, .. }) => *scope_id,
            Type::LambdaFunction(CheckedLambdaFunctionNode { scope_id, .. }) => *scope_id,
            Type::Felt(_) => ScopeId::primitive(),
            Type::Bool(_) => ScopeId::primitive(),
            Type::U32(_) => ScopeId::primitive(),
            Type::TypeVariable(CheckedTypeVariableNode { scope_id, .. }) => *scope_id,
            _ => panic!("Type::scope_id called on non-composite type: {:?}", self),
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
            Type::Array(CheckedArrayNode {
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
            Type::Tuple(_) => panic!("Tuple types do not support trait implementations"),
            Type::U32(CheckedU32Node {
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
            Type::U32(CheckedU32Node {
                implementations, ..
            }) => implementations,
            Type::Array(CheckedArrayNode {
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

    pub fn name(&self) -> IdentId {
        match self {
            Type::Struct(CheckedStructNode { name, .. }) => *name,
            Type::Enum(CheckedEnumNode { name, .. }) => *name,
            Type::Function(CheckedFunctionNode { name, .. }) => *name,
            Type::Trait(CheckedTraitNode { name, .. }) => *name,
            Type::Const(CheckedConstNode { name, .. }) => name.unwrap(),
            Type::Array(_) => IdentId::TYPE_ARRAY,
            _ => unreachable!(),
        }
    }

    pub fn body(&self) -> ExprId {
        match self {
            Type::Function(CheckedFunctionNode { body, .. }) => body.unwrap(),
            Type::LambdaFunction(CheckedLambdaFunctionNode { body, .. }) => body.clone(),
            _ => unreachable!(),
        }
    }

    pub fn generic_parameters(&self) -> Vec<TypeId> {
        match self {
            Type::Struct(CheckedStructNode {
                generic_parameters, ..
            }) => generic_parameters.to_vec(),
            Type::Enum(CheckedEnumNode {
                generic_parameters, ..
            }) => generic_parameters.to_vec(),
            Type::Function(CheckedFunctionNode {
                generic_parameters, ..
            }) => generic_parameters.to_vec(),
            Type::Trait(CheckedTraitNode {
                generic_parameters, ..
            }) => generic_parameters.to_vec(),
            Type::Array(CheckedArrayNode { inner_ty, size, .. }) => {
                vec![inner_ty.clone(), size.clone()]
            }
            Type::LambdaFunction(_) => vec![],
            Type::FunctionSignature(_) => vec![],
            Type::GenericInstance(_, generic_parameters) => generic_parameters.to_vec(),
            _ => unreachable!(),
        }
    }

    pub fn parameters(&self) -> Vec<(IdentId, TypeQualifier, TypeId)> {
        match self {
            Type::Function(CheckedFunctionNode { parameters, .. }) => parameters.to_vec(),
            Type::LambdaFunction(CheckedLambdaFunctionNode { parameters, .. }) => {
                parameters.to_vec()
            }
            _ => unreachable!(),
        }
    }

    pub fn signature(&self) -> CheckedFunctionSignature {
        match self {
            Type::Function(f) => f.signature(),
            Type::LambdaFunction(l) => l.signature(),
            Type::FunctionSignature(s) => s.clone(),
            _ => unreachable!(),
        }
    }

    pub fn kind(&self) -> TypeKind {
        match self {
            Type::Unknown => TypeKind::Unknown,
            Type::VOID => TypeKind::VOID,
            Type::Felt(_) => TypeKind::Felt,
            Type::Bool(_) => TypeKind::Bool,
            Type::U32(_) => TypeKind::U32,
            Type::Array(_) => TypeKind::Array,
            Type::Struct(_) => TypeKind::Struct,
            Type::Enum(_) => TypeKind::Enum,
            Type::Tuple(_) => TypeKind::Tuple,
            Type::Function(_) => TypeKind::Function,
            Type::Trait(_) => TypeKind::Trait,
            Type::Const(_) => TypeKind::Const,
            Type::LambdaFunction(_) => TypeKind::LambdaFunction,
            Type::FunctionSignature(_) => TypeKind::FunctionSignature,
            Type::TypeVariable(_) => TypeKind::TypeVariable,
            Type::GenericInstance(_, _) => TypeKind::GenericInstance,
        }
    }
}
