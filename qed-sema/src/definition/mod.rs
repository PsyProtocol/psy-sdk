mod array;
mod r#const;
mod r#enum;
mod function;
mod r#impl;
mod r#struct;
mod r#trait;
mod type_alias;

pub use array::*;
use enum_as_inner::EnumAsInner;
pub use function::*;
use qed_ast::{NodeInfo, NodeType};
pub use r#const::*;
pub use r#enum::*;
pub use r#impl::*;
pub use r#struct::*;
pub use r#trait::*;
pub use type_alias::*;

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum CheckedDefinitionNode {
    Function(CheckedFunctionNode),
    Struct(CheckedStructNode),
    Enum(CheckedEnumNode),
    Impl(CheckedImplNode),
    Trait(CheckedTraitNode),
    TypeAlias(CheckedTypeAliasNode),
    Const(CheckedConstNode),
}

impl NodeInfo for CheckedDefinitionNode {
    fn node_type(&self) -> NodeType {
        match self {
            Self::Function(node) => node.node_type(),
            Self::Struct(node) => node.node_type(),
            Self::Enum(node) => node.node_type(),
            Self::Impl(node) => node.node_type(),
            Self::Trait(node) => node.node_type(),
            Self::TypeAlias(node) => node.node_type(),
            Self::Const(node) => node.node_type(),
        }
    }
}
