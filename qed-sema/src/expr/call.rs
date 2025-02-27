use qed_ast::{ExprId, NodeInfo, NodeType};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedCallNode {
    pub callee: ExprId,
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
    pub callee: ExprId,
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

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedTupleAccessNode {
    pub value: ExprId,   // `ExprId` for the tuple being accessed
    pub index: usize,    // The index of the tuple element being accessed
    pub type_id: TypeId, // The type of the element at `index`
}

impl NodeInfo for CheckedTupleAccessNode {
    fn node_type(&self) -> NodeType {
        NodeType::TupleAccessExpr
    }
}
