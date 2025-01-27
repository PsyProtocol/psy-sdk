use qed_ast::{ExprId, IdentId, NodeType};

use crate::{ScopeId, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedVariableNode {
    pub name: IdentId,
    pub ty: TypeId,
    pub mutable: bool,
    pub cnst: bool,
    pub value: ExprId,
    pub scope_id: ScopeId,
}

impl CheckedVariableNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::VariableStmt
    }
}
