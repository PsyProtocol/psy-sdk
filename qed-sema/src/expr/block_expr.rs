use qed_ast::{ExprId, NodeInfo, NodeType, Span, StmtId};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedBlockExprNode {
    pub stmts: Vec<StmtId>,
    pub expr: Option<ExprId>,
    pub type_id: TypeId,
    pub scope_id: ScopeId,
    pub span: Span,
}

impl NodeInfo for CheckedBlockExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::BlockExpr
    }
}
