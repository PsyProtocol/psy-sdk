use crate::{ExprId, IdentId, NodeInfo, NodeType, Span, TypeQualifier, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaFunctionNode {
    pub parameters: Vec<(IdentId, TypeQualifier, UncheckedType)>,
    pub body: ExprId,
    pub return_type: Option<UncheckedType>,
    pub span: Span,
}

impl NodeInfo for LambdaFunctionNode {
    fn node_type(&self) -> NodeType {
        NodeType::LambdaFunctionExpr
    }
}
