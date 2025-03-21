use qed_ast::{Location, NodeInfo, NodeType};

use crate::{TypeId, VarId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedPathNode {
    pub variable: Option<VarId>,
    pub type_id: TypeId,
    pub location: Location,
}

impl CheckedPathNode {
    pub fn new(variable: Option<VarId>, type_id: TypeId, location: Location) -> Self {
        Self {
            variable,
            type_id,
            location,
        }
    }
}

impl NodeInfo for CheckedPathNode {
    fn node_type(&self) -> NodeType {
        NodeType::PathExpr
    }
}
