use qed_ast::{ExprId, NodeInfo, NodeType, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedReturnNode {
    pub ret: Option<ExprId>,
    pub span: Span,
}

impl NodeInfo for CheckedReturnNode {
    fn node_type(&self) -> NodeType {
        NodeType::ReturnStmt
    }
}
