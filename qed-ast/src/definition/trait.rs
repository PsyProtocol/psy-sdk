use crate::{AstVisitor, DefId, FunctionNode, IdentId, NodeInfo, NodeType, Visibility};

#[derive(Clone, Debug, PartialEq)]
pub struct TraitNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub body: Vec<DefId>,
    pub visibility: Visibility,
}

impl NodeInfo for TraitNode {
    fn node_type(&self) -> NodeType {
        NodeType::TraitDef
    }
}
