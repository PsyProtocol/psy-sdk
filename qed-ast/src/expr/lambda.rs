use crate::{IdentId, NodeInfo, NodeType, StmtId, TypeQualifier, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaFunctionNode {
    pub parameters: Vec<(IdentId, TypeQualifier, UncheckedType)>,
    pub body: StmtId,
    pub return_type: Option<UncheckedType>,
}

impl NodeInfo for LambdaFunctionNode {
    fn node_type(&self) -> NodeType {
        NodeType::LambdaFunctionExpr
    }
}
