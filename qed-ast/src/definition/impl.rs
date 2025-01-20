use crate::{AstVisitor, DefId, FunctionNode, IdentId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct ImplNode {
    pub generic_parameters: Vec<IdentId>,
    pub trait_name: Option<IdentId>,
    pub ty: IdentId,
    pub body: Vec<DefId>,
}

// impl ImplNode {
//     pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
//         &self,
//         visitor: &mut V,
//         ctx: &mut V::Context,
//     ) -> Result<V::StmtResult, V::Error> {
//         visitor.visit_impl(self, ctx)
//     }
// }
