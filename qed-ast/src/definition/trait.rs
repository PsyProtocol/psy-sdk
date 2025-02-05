use crate::{AstVisitor, FunctionNode, IdentId, NodeType, Visibility};

#[derive(Clone, Debug, PartialEq)]
pub struct TraitNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub body: Vec<FunctionNode>,
    pub visibility: Visibility,
}

impl TraitNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::TraitDef
    }
}
