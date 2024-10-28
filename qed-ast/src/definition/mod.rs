pub mod r#enum;
pub mod function;
pub mod r#impl;
pub mod r#struct;

pub use function::*;
pub use r#enum::*;
pub use r#impl::*;
pub use r#struct::*;

use crate::AstVisitor;
use strum::EnumIs;

#[derive(Debug, Clone, PartialEq, EnumIs)]
pub enum DefinitionNode {
    Function(FunctionNode),
    Struct(StructNode),
    Enum(EnumNode),
    Impl(ImplNode),
}

impl DefinitionNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) {
        visitor.visit_definition(self)
    }
}
