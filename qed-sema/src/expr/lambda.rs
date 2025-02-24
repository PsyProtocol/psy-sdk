use qed_ast::TypeQualifier;

use crate::{CheckedFunctionSignature, IdentId, NodeInfo, NodeType, ScopeId, StmtId, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedLambdaFunctionNode {
    pub parameters: Vec<(IdentId, TypeQualifier, TypeId)>,
    pub body: StmtId,
    pub captures: Vec<(ScopeId, IdentId)>,
    pub return_type: Option<TypeId>,
    pub type_id: TypeId,
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
