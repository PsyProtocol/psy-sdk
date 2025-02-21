use qed_ast::{AttrNode, IdentId, NodeInfo, NodeType, StmtId, Visibility};

use crate::{ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedFunctionNode {
    pub name: IdentId,
    pub parameters: Vec<(IdentId, bool, TypeId)>,
    pub generic_parameters: Vec<TypeId>,
    pub body: Option<StmtId>,
    pub return_type: TypeId,
    pub scope_id: ScopeId,
    pub visibility: Visibility,
    pub attrs: Vec<AttrNode>,
}

impl CheckedFunctionNode {
    pub fn signature(&self) -> CheckedFunctionSignature {
        CheckedFunctionSignature {
            parameters: self
                .parameters
                .iter()
                .map(|(_, mutable, ty)| (mutable.clone(), ty.clone()))
                .collect(),
            return_type: self.return_type,
        }
    }
}

impl NodeInfo for CheckedFunctionNode {
    fn node_type(&self) -> NodeType {
        NodeType::FunctionDef
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedFunctionSignature {
    pub parameters: Vec<(bool, TypeId)>,
    pub return_type: TypeId,
}
