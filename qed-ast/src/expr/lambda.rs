use crate::{ExprId, IdentId, NodeInfo, NodeType, TypeQualifier, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaFunctionNode {
    pub parameters: Vec<(IdentId, TypeQualifier, UncheckedType)>,
    pub body: ExprId,
    pub return_type: Option<UncheckedType>,
}

impl NodeInfo for LambdaFunctionNode {
    fn node_type(&self) -> NodeType {
        NodeType::LambdaFunctionExpr
    }
}
