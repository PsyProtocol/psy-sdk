use qed_ast::{NodeInfo, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedBlockNode {
    pub stmts: Vec<StmtId>,
}

impl NodeInfo for CheckedBlockNode {
    fn node_type(&self) -> NodeType {
        NodeType::BlockStmt
    }
}
