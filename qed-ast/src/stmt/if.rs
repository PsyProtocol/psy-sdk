use crate::{AstVisitor, BlockNode, ExprId, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub predicate: ExprId,
    pub body: StmtId,
}

impl Case {
    pub fn new(predicate: ExprId, body: StmtId) -> Self {
        Self { predicate, body }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfNode {
    pub if_branch: Case,
    pub elseif_branch: Vec<Case>,
    pub else_branch: Option<StmtId>,
}

// impl IfNode {
// pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
//     &self,
//     visitor: &mut V,
//     ctx: &mut V::Context,
// ) -> Result<V::StmtResult, V::Error> {
//     visitor.visit_if(self, ctx)
// }
// }
