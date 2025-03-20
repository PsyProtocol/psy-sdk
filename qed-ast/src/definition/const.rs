use crate::{ExprId, IdentId, Location, NodeInfo, NodeType, UncheckedType, Visibility};

#[derive(Clone, Debug, PartialEq)]
pub struct ConstNode {
    pub name: IdentId,
    pub ty: UncheckedType,
    pub value: ExprId,
    pub visibility: Visibility,
    pub location: Location,
}

impl NodeInfo for ConstNode {
    fn node_type(&self) -> NodeType {
        NodeType::ConstDef
    }
}
