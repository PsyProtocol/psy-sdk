use crate::{ExprId, IdentId, NodeInfo, NodeType, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct VariableNode {
    pub name: IdentId,
    pub ty: UncheckedType,
    pub mutable: bool,
    pub value: ExprId,
}

impl NodeInfo for VariableNode {
    fn node_type(&self) -> NodeType {
        NodeType::VariableStmt
    }
}
