use crate::{AstVisitor, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockNode {
    pub stmts: Vec<StmtId>,
}
