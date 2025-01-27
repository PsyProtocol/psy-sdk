use qed_ast::{BlockNode, IdentId, NodeType};

use crate::{CheckedBlockNode, ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedFunctionNode {
    pub name: IdentId,
    pub parameters: Vec<(IdentId, bool, TypeId)>,
    pub generic_parameters: Vec<TypeId>,
    pub body: Option<CheckedBlockNode>,
    pub return_type: Option<TypeId>,
    pub scope_id: ScopeId,
    pub is_pub: bool,
}

impl CheckedFunctionNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::FunctionDef
    }
}
