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

use crate::{AstVisitor, IdentId, NodeType};
use strum::EnumIs;

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum DefinitionNode {
    Function(FunctionNode),
    Struct(StructNode),
    Enum(EnumNode),
    Impl(ImplNode),
    Trait(TraitNode),
}

impl DefinitionNode {
    pub fn node_type(&self) -> NodeType {
        match self {
            DefinitionNode::Function(_) => NodeType::FunctionDef,
            DefinitionNode::Struct(_) => NodeType::StructDef,
            DefinitionNode::Enum(_) => NodeType::EnumDef,
            DefinitionNode::Impl(_) => NodeType::ImplDef,
            DefinitionNode::Trait(_) => NodeType::TraitDef,
        }
    }

    // pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
    //     &self,
    //     visitor: &mut V,
    //     ctx: &mut V::Context,
    // ) -> Result<V::StmtResult, V::Error> {
    //     visitor.visit_definition(self, ctx)
    // }
}
