use crate::{AstVisitor, FunctionNode, IdentId, NodeType};

#[derive(Clone, Debug, PartialEq)]
pub struct TraitNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub body: Vec<FunctionNode>,
}

impl TraitNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::TraitDef
    }
}
