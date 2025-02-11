use qed_ast::{ExprId, NodeInfo, NodeType, StmtId};

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedBlockExprNode {
    pub stmts: Vec<StmtId>,
    pub return_expr: Option<ExprId>,
    pub type_id: TypeId,
}

impl NodeInfo for CheckedBlockExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::BlockExpr
    }
}
