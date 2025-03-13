use crate::{ExprId, IdentId, NodeInfo, NodeType, Span, TypeQualifier, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct VariableNode {
    pub name: IdentId,
    pub ty: UncheckedType,
    pub qualifier: TypeQualifier,
    pub value: ExprId,
    pub span: Span,
}

impl NodeInfo for VariableNode {
    fn node_type(&self) -> NodeType {
        NodeType::VariableStmt
    }
}
