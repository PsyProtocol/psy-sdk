use crate::{
    Comment, DefId, GenericParameter, Identifier, Location, NodeInfo, NodeType, Visibility,
};

#[derive(Clone, Debug, PartialEq)]
pub struct TraitNode {
    pub name: Identifier,
    pub generic_parameters: Vec<GenericParameter>,
    pub body: Vec<DefId>,
    pub visibility: Visibility,
    pub comments: Vec<Comment>,
    pub location: Location,
}

impl NodeInfo for TraitNode {
    fn node_type(&self) -> NodeType {
        NodeType::TraitDef
    }
}
