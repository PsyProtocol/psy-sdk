use crate::{IdentId, NodeInfo, NodeType, StmtId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct ClosureNode {
    pub parameters: Vec<(IdentId, bool, UncheckedType)>,
    pub body: StmtId,
    pub return_type: Option<UncheckedType>,
}

impl NodeInfo for ClosureNode {
    fn node_type(&self) -> NodeType {
        NodeType::ClosureExpr
    }
}
