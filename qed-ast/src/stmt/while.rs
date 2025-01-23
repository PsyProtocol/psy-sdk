use crate::{AstVisitor, BlockNode, ExprId, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct WhileNode {
    pub predicate: ExprId,
    pub body: StmtId,
}
