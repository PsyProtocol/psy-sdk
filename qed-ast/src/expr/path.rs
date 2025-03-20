use crate::{IdentId, Identifier, Location, NodeInfo, NodeType, UncheckedType};

#[derive(Clone, Debug, PartialEq)]
pub struct PathNode {
    pub root: Option<UncheckedType>,
    pub segments: Vec<Identifier>,
    pub target: Identifier,
    pub location: Location,
}

impl PathNode {
    pub fn from_target(target: Identifier, location: Location) -> Self {
        Self {
            root: None,
            segments: vec![],
            target,
            location,
        }
    }

    pub fn is_receiver(&self) -> bool {
        self.root.is_none() && self.target.id == IdentId::SELF && self.segments.is_empty()
    }
}

impl NodeInfo for PathNode {
    fn node_type(&self) -> NodeType {
        NodeType::PathExpr
    }
}
