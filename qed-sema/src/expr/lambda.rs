use qed_ast::{ExprId, Span, TypeQualifier};

use crate::{CheckedFunctionSignature, IdentId, NodeInfo, NodeType, ScopeId, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedLambdaFunctionNode {
    pub name: IdentId,
    pub parameters: Vec<(IdentId, TypeQualifier, TypeId)>,
    pub body: ExprId,
    pub return_type: TypeId,
    pub scope_id: ScopeId,
    pub type_id: TypeId,
    pub span: Span,
}

impl CheckedLambdaFunctionNode {
    pub fn signature(&self) -> CheckedFunctionSignature {
        CheckedFunctionSignature {
            parameters: self
                .parameters
                .iter()
                .map(|(_, _, ty)| ty.clone())
                .collect(),
            return_type: self.return_type.clone(),
        }
    }
}

impl NodeInfo for CheckedLambdaFunctionNode {
    fn node_type(&self) -> NodeType {
        NodeType::LambdaFunctionExpr
    }
}
