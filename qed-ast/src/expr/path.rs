use crate::{
    AstVisitor, {ExprId, IdentId},
};

#[derive(Clone, Debug, PartialEq)]
pub struct PathNode(pub IdentId);

// impl PathNode {
//     pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
//         &self,
//         visitor: &mut V,
//         ctx: &mut V::Context,
//     ) -> Result<V::ExprResult, V::Error> {
//         visitor.visit_path(self, ctx)
//     }
// }
