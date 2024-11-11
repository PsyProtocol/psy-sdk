use std::collections::HashMap;

use crate::{AstVisitor, FunctionNode, IdentId, UncheckedType};

#[derive(Clone, Debug, PartialEq)]
pub enum EnumVariant {
    Basic(IdentId),
    Tuple(IdentId, Vec<UncheckedType>),
    Struct(IdentId, Vec<(IdentId, UncheckedType)>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub variants: Vec<EnumVariant>,
}

impl EnumNode {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &self,
        visitor: &mut V,
    ) -> V::StmtResult {
        visitor.visit_enum(self)
    }
}
