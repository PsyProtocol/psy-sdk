mod attr;
mod r#enum;
mod function;
mod r#impl;
mod r#struct;
mod r#trait;

pub use attr::*;
use enum_as_inner::EnumAsInner;
pub use function::*;
pub use r#enum::*;
pub use r#impl::*;
pub use r#struct::*;
pub use r#trait::*;

use crate::{AstVisitor, IdentId, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum DefinitionNode {
    Function(FunctionNode),
    Struct(StructNode),
    Enum(EnumNode),
    Impl(ImplNode),
    Trait(TraitNode),
}

impl NodeInfo for DefinitionNode {
    fn node_type(&self) -> NodeType {
        match self {
            Self::Function(node) => node.node_type(),
            Self::Struct(node) => node.node_type(),
            Self::Enum(node) => node.node_type(),
            Self::Impl(node) => node.node_type(),
            Self::Trait(node) => node.node_type(),
        }
    }
}
