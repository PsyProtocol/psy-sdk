use crate::{AstVisitor, FunctionNode, IdentId};

#[derive(Clone, Debug, PartialEq)]
pub struct TraitNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub body: Vec<FunctionNode>,
}

// impl TraitNode {
//     pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
//         &self,
//         visitor: &mut V,
//         ctx: &mut V::Context,
//     ) -> Result<V::StmtResult, V::Error> {
//         visitor.visit_trait(self, ctx)
//     }
// }
