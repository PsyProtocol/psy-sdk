use qed_ast::{ExprId, NodeInfo, NodeType, Span};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedCallNode {
    pub callee: ExprId,
    pub generic_parameters: Vec<TypeId>,
    pub args: Vec<ExprId>,
    pub type_id: TypeId,
    pub span: Span,
}

impl NodeInfo for CheckedCallNode {
    fn node_type(&self) -> NodeType {
        NodeType::CallExpr
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedMemberCallNode {
    pub callee: ExprId,
    pub receiver: ExprId,
    pub generic_parameters: Vec<TypeId>,
    pub args: Vec<ExprId>,
    pub type_id: TypeId,
    pub span: Span,
}

impl NodeInfo for CheckedMemberCallNode {
    fn node_type(&self) -> NodeType {
        NodeType::MemberCallExpr
    }
}
