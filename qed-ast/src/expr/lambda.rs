use crate::{ExprId, FunctionParameter, NodeInfo, NodeType, Span, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaFunctionNode {
    pub parameters: Vec<FunctionParameter>,
    pub body: ExprId,
    pub return_type: Option<UncheckedType>,
    pub span: Span,
}

impl NodeInfo for LambdaFunctionNode {
    fn node_type(&self) -> NodeType {
        NodeType::LambdaFunctionExpr
    }
}
