use qed_ast::{ExprId, IdentId, NodeInfo, NodeType, TypeQualifier};

use crate::{ScopeId, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedVariableNode {
    pub name: IdentId,
    pub ty: TypeId,
    pub qualifier: TypeQualifier,
    pub value: ExprId,
    pub scope_id: ScopeId,
}

impl NodeInfo for CheckedVariableNode {
    fn node_type(&self) -> NodeType {
        NodeType::VariableStmt
    }
}
