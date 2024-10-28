use crate::{arena::ExprId, visitor::AstVisitor, BlockNode};

#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub predicate: ExprId,
    pub body: BlockNode,
}

impl Case {
    pub fn new(predicate: ExprId, body: BlockNode) -> Self {
        Self { predicate, body }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfNode {
    pub if_branch: Case,
    pub elseif_branch: Vec<Case>,
    pub else_branch: Option<BlockNode>,
}

impl IfNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::StmtResult {
        visitor.visit_if(self)
    }
}
