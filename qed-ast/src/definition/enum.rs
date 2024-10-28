use std::collections::HashMap;

use crate::{arena::IdentId, AstVisitor, FunctionNode, Type};

#[derive(Clone, Debug, PartialEq)]
pub enum EnumVariant {
    Basic(IdentId),
    Tuple(IdentId, Vec<Type>),
    Struct(IdentId, Vec<(IdentId, Type)>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub variants: Vec<EnumVariant>,
    pub functions: HashMap<IdentId, FunctionNode>,
}

impl EnumNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::StmtResult {
        visitor.visit_enum(self)
    }
}
