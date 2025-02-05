use crate::{AstVisitor, ExprId, NodeInfo, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnNode(pub Option<ExprId>);

impl NodeInfo for ReturnNode {
    fn node_type(&self) -> NodeType {
        NodeType::ReturnStmt
    }
}
