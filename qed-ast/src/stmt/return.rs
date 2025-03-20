use crate::{ExprId, Location, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnNode {
    pub expr_id: Option<ExprId>,
    pub location: Location,
}

impl NodeInfo for ReturnNode {
    fn node_type(&self) -> NodeType {
        NodeType::ReturnStmt
    }
}
