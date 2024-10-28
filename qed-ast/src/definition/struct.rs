use std::collections::HashMap;

use crate::{arena::IdentId, AstVisitor, DefId, FunctionNode, Type, ValueNode};

#[derive(Clone, Debug, PartialEq)]
pub struct StructNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub fields: Vec<(IdentId, Type)>,
    pub functions: HashMap<IdentId, FunctionNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructInstance<F> {
    pub name: IdentId,
    pub def_id: DefId,
    pub generic_parameters: Vec<DefId>,
    pub fields: HashMap<IdentId, ValueNode<F>>,
}

impl StructNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::StmtResult {
        visitor.visit_struct(self)
    }
}
