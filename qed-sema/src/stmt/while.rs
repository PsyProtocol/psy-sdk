use qed_ast::{ExprId, Location, NodeInfo, NodeType};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedWhileNode {
    pub predicate: ExprId,
    pub type_id: TypeId,
    pub body: ExprId,
    pub location: Location,
}

impl NodeInfo for CheckedWhileNode {
    fn node_type(&self) -> NodeType {
        NodeType::WhileStmt
    }
}
