use crate::{ExprId, IdentId, Location, NodeInfo, NodeType, TypeQualifier, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct VariableNode {
    pub name: IdentId,
    pub ty: UncheckedType,
    pub qualifier: TypeQualifier,
    pub value: ExprId,
    pub location: Location,
}

impl NodeInfo for VariableNode {
    fn node_type(&self) -> NodeType {
        NodeType::VariableStmt
    }
}
