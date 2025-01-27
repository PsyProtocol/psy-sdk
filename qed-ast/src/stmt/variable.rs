use crate::{AstVisitor, ExprId, IdentId, NodeType, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct VariableNode {
    pub name: IdentId,
    pub ty: UncheckedType,
    pub mutable: bool,
    pub cnst: bool,
    pub value: ExprId,
}

impl VariableNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::VariableStmt
    }
}
