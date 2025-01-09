pub mod r#enum;
pub mod function;
pub mod r#impl;
pub mod r#struct;

pub use function::*;
pub use r#enum::*;
pub use r#impl::*;
pub use r#struct::*;

use crate::{AstVisitor, IdentId};
use strum::EnumIs;

#[derive(Debug, Clone, PartialEq, EnumIs)]
pub enum DefinitionNode {
    Function(FunctionNode),
    Struct(StructNode),
    Enum(EnumNode),
    Impl(ImplNode),
}

impl DefinitionNode {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &self,
        visitor: &mut V,
    ) -> V::StmtResult {
        visitor.visit_definition(self)
    }
}
