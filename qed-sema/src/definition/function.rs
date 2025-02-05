use qed_ast::{BlockNode, IdentId, NodeInfo, NodeType, StmtId, Visibility};

use crate::{CheckedBlockNode, ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedFunctionNode {
    pub name: IdentId,
    pub parameters: Vec<(IdentId, bool, TypeId)>,
    pub generic_parameters: Vec<TypeId>,
    pub body: Option<StmtId>,
    pub return_type: Option<TypeId>,
    pub scope_id: ScopeId,
    pub visibility: Visibility,
}

impl NodeInfo for CheckedFunctionNode {
    fn node_type(&self) -> NodeType {
        NodeType::FunctionDef
    }
}
