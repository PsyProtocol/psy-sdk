use qed_ast::{ExprId, NodeInfo, NodeType};

use crate::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedCallNode {
    pub variable: ExprId,
    pub receiver: Option<ExprId>,
    pub generic_parameters: Vec<TypeId>,
    pub args: Vec<ExprId>,
    pub type_id: TypeId,
}

impl NodeInfo for CheckedCallNode {
    fn node_type(&self) -> NodeType {
        NodeType::CallExpr
    }
}
