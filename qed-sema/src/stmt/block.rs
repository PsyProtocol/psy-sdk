use qed_ast::{NodeInfo, NodeType, StmtId};

use crate::ScopeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedBlockNode {
    pub stmts: Vec<StmtId>,
    pub scope_id: ScopeId,
}

impl NodeInfo for CheckedBlockNode {
    fn node_type(&self) -> NodeType {
        NodeType::BlockStmt
    }
}
