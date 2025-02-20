use crate::{NodeInfo, NodeType, StmtId};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExprNode {
    pub stmts: Vec<StmtId>,
    pub uses: Vec<StmtId>,
}

impl NodeInfo for BlockExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::BlockExpr
    }
}
