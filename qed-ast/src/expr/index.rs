use crate::{AstVisitor, ExprId, IdentId};

#[derive(Clone, Debug, PartialEq)]
pub struct IndexAccessNode {
    pub value: ExprId,
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemberAccessNode {
    pub value: ExprId,
    pub field: IdentId,
}

impl IndexAccessNode {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &self,
        visitor: &mut V,
        ctx: &mut V::Context,
    ) -> Result<V::ExprResult, V::Error> {
        visitor.visit_index_access(self, ctx)
    }
}

impl MemberAccessNode {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &self,
        visitor: &mut V,
        ctx: &mut V::Context,
    ) -> Result<V::ExprResult, V::Error> {
        visitor.visit_member_access(self, ctx)
    }
}
