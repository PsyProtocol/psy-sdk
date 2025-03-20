use qed_ast::{ExprId, IdentId, Location, NodeInfo, NodeType, TypeQualifier};

use crate::{ScopeId, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedVariableNode {
    pub name: IdentId,
    pub ty: TypeId,
    pub qualifier: TypeQualifier,
    pub value: ExprId,
    pub scope_id: ScopeId,
    pub location: Location,
}

impl NodeInfo for CheckedVariableNode {
    fn node_type(&self) -> NodeType {
        NodeType::VariableStmt
    }
}
