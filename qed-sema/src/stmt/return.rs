use qed_ast::{ExprId, Location, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedReturnNode {
    pub ret: Option<ExprId>,
    pub location: Location,
}

impl NodeInfo for CheckedReturnNode {
    fn node_type(&self) -> NodeType {
        NodeType::ReturnStmt
    }
}
