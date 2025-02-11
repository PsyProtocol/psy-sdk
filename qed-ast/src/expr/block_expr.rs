use std::fmt::Display;
use crate::{AstVisitor, BlockNode, ExprId, NodeInfo, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExprNode {
    pub stmts: Vec<StmtId>,
    pub return_expr: Option<ExprId>

}

impl NodeInfo for BlockExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::BlockExpr
    }
}

impl From<BlockNode> for BlockExprNode {
    fn from(block: BlockNode) -> Self {
        Self {
            stmts: block.stmts.clone(),
            return_expr: None,
        }
    }
}
