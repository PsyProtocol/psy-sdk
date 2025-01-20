use std::collections::HashMap;

use crate::{AstVisitor, AttrNode, FunctionNode, IdentId, UncheckedType, ValueNode};

#[derive(Clone, Debug, PartialEq)]
pub struct StructNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub fields: Vec<(IdentId, UncheckedType)>,
    pub attrs: Vec<AttrNode>,
}

// impl StructNode {
//     pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
//         &self,
//         visitor: &mut V,
//         ctx: &mut V::Context,
//     ) -> Result<V::StmtResult, V::Error> {
//         visitor.visit_struct(self, ctx)
//     }
// }
