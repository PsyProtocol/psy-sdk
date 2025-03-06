use crate::{IdentId, NodeInfo, NodeType, UncheckedType};

#[derive(Clone, Debug, PartialEq)]
pub struct PathNode {
    pub root: Option<UncheckedType>,
    pub segments: Vec<IdentId>,
    pub target: IdentId,
}

impl PathNode {
    pub fn from_target(target: IdentId) -> Self {
        Self {
            root: None,
            segments: vec![],
            target,
        }
    }

    pub fn new(root: Option<UncheckedType>, target: IdentId, segments: Vec<IdentId>) -> Self {
        Self {
            root,
            segments,
            target,
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
