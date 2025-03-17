use crate::{IdentId, NodeInfo, NodeType, Span, UncheckedType};

#[derive(Clone, Debug, PartialEq)]
pub struct PathNode {
    pub root: Option<UncheckedType>,
    pub segments: Vec<IdentId>,
    pub target: IdentId,
    pub span: Span,
}

impl PathNode {
    pub fn from_target(target: IdentId, span: Span) -> Self {
        Self {
            root: None,
            segments: vec![],
            target,
            span,
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
