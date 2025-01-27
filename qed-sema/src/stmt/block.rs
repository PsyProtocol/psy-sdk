use qed_ast::{NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedBlockNode {
    pub stmts: Vec<StmtId>,
}

impl CheckedBlockNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::BlockStmt
    }
}
