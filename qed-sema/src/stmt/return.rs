use qed_ast::{ExprId, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedReturnNode {
    pub ret: Option<ExprId>,
}

impl NodeInfo for CheckedReturnNode {
    fn node_type(&self) -> NodeType {
        NodeType::ReturnStmt
    }
}
