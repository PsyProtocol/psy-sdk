use qed_ast::Visibility;

use crate::{ExprId, IdentId, NodeInfo, NodeType, ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedConstNode {
    pub name: IdentId,
    pub ty: TypeId,
    pub value: ExprId,
    pub visibility: Visibility,
    pub scope_id: ScopeId,
}

impl NodeInfo for CheckedConstNode {
    fn node_type(&self) -> NodeType {
        NodeType::ConstDef
    }
}
