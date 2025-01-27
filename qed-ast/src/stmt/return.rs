use crate::{AstVisitor, ExprId, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnNode(pub Option<ExprId>);

impl ReturnNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::ReturnStmt
    }
}
