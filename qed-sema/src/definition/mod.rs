mod array;
mod r#enum;
mod function;
mod r#impl;
mod r#struct;
mod r#trait;

pub use array::*;
use enum_as_inner::EnumAsInner;
pub use function::*;
use qed_ast::NodeType;
pub use r#enum::*;
pub use r#impl::*;
pub use r#struct::*;
pub use r#trait::*;

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum CheckedDefinitionNode {
    Function(CheckedFunctionNode),
    Struct(CheckedStructNode),
    Enum(CheckedEnumNode),
    Impl(CheckedImplNode),
    Trait(CheckedTraitNode),
}

impl CheckedDefinitionNode {
    pub fn node_type(&self) -> NodeType {
        match self {
            Self::Function(node) => node.node_type(),
            Self::Struct(node) => node.node_type(),
            Self::Enum(node) => node.node_type(),
            Self::Impl(node) => node.node_type(),
            Self::Trait(node) => node.node_type(),
        }
    }
}
