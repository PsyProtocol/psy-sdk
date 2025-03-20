use crate::{IdentId, Location, NodeInfo, NodeType, UncheckedType};

#[derive(Clone, Debug, PartialEq)]
pub struct PathNode {
    pub root: Option<UncheckedType>,
    pub segments: Vec<IdentId>,
    pub target: IdentId,
    pub location: Location,
}

impl PathNode {
    pub fn from_target(target: IdentId, location: Location) -> Self {
        Self {
            root: None,
            segments: vec![],
            target,
            location,
        }
    }

    pub fn is_receiver(&self) -> bool {
        self.root.is_none() && self.target == IdentId::SELF && self.segments.is_empty()
    }
}

impl NodeInfo for PathNode {
    fn node_type(&self) -> NodeType {
        NodeType::PathExpr
    }
}
