use crate::{AstVisitor, ExprId, PathNode, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct CallNode {
    pub variable: ExprId,
    pub receiver: Option<ExprId>,
    pub generic_parameters: Vec<UncheckedType>,
    pub args: Vec<ExprId>,
}

// impl CallNode {
//     pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
//         &self,
//         visitor: &mut V,
//         ctx: &mut V::Context,
//     ) -> Result<V::ExprResult, V::Error> {
//         visitor.visit_call(self, ctx)
//     }
// }
