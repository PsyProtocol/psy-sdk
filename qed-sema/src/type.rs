use std::fmt::{Display, Formatter};

use qed_ast::IdentId;

use crate::{CheckedEnumNode, CheckedFunctionNode, CheckedImplNode, CheckedStructNode};
use qed_common::define_arena_id;
use strum::{EnumIs, EnumTryAs};

define_arena_id!(TypeId);

impl Display for TypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub const UNKOWN_TYPE: TypeId = TypeId(0);
pub const BOOL_TYPE: TypeId = TypeId(1);
pub const FELT_TYPE: TypeId = TypeId(2);
pub const VOID_TYPE: TypeId = TypeId(3);

#[derive(Debug, Clone, PartialEq, EnumIs, EnumTryAs)]
pub enum Type {
    Unknown,
    Felt,
    Bool,
    Struct(CheckedStructNode),
    Enum(CheckedEnumNode),
    Function(CheckedFunctionNode),
    Impl(CheckedImplNode),
    TypeVariable(IdentId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeKey {
    pub id: IdentId,
    pub generic_parameters: Vec<TypeId>,
}

impl TypeKey {
    pub fn new(id: IdentId, generic_parameters: Vec<TypeId>) -> Self {
        Self {
            id,
            generic_parameters,
        }
    }

    pub fn id(&self) -> IdentId {
        self.id
    }

    pub fn has_generics(&self) -> bool {
        !self.generic_parameters.is_empty()
    }
}

impl From<IdentId> for TypeKey {
    fn from(value: IdentId) -> Self {
        TypeKey::new(value, vec![])
    }
}

impl Type {
    pub fn key(&self) -> TypeKey {
        let (id, generic_parameters) = match self {
            Type::Unknown => (IdentId::TYPE_UNKNOWN, vec![]),
            Type::Felt => (IdentId::TYPE_FELT, vec![]),
            Type::Bool => (IdentId::TYPE_BOOL, vec![]),
            Type::Struct(CheckedStructNode {
                name,
                generic_parameters,
                ..
            }) => (name.clone(), generic_parameters.clone()),
            Type::Enum(CheckedEnumNode {
                name,
                generic_parameters,
                ..
            }) => (name.clone(), generic_parameters.clone()),
            Type::Function(CheckedFunctionNode {
                name,
                generic_parameters,
                ..
            }) => (name.clone(), generic_parameters.clone()),
            Type::Impl(CheckedImplNode {
                generic_parameters,
                ty,
                ..
            }) => (ty.clone(), generic_parameters.clone()),
            Type::TypeVariable(id) => panic!("Type::id called on TypeVariable type"),
        };
        TypeKey::new(id, generic_parameters)
    }
}
