use qed_ast::{ExprId, NodeInfo, NodeType};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedCallNode {
    pub variable: ExprId,
    pub generic_parameters: Vec<TypeId>,
    pub args: Vec<ExprId>,
    pub type_id: TypeId,
}

impl NodeInfo for CheckedCallNode {
    fn node_type(&self) -> NodeType {
        NodeType::CallExpr
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedMemberCallNode {
    pub variable: ExprId,
    pub receiver: ExprId,
    pub generic_parameters: Vec<TypeId>,
    pub args: Vec<ExprId>,
    pub type_id: TypeId,
}

impl NodeInfo for CheckedMemberCallNode {
    fn node_type(&self) -> NodeType {
        NodeType::MemberCallExpr
    }
}
