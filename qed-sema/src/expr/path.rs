use qed_ast::{NodeInfo, NodeType, Span};

use crate::{TypeId, VarId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedPathNode {
    pub variable: Option<VarId>,
    pub root: Option<TypeId>,
    pub type_id: TypeId,
    pub span: Span,
}

impl CheckedPathNode {
    pub fn new(variable: Option<VarId>, root: Option<TypeId>, type_id: TypeId, span: Span) -> Self {
        Self {
            variable,
            root,
            type_id,
            span,
        }
    }
}

impl NodeInfo for CheckedPathNode {
    fn node_type(&self) -> NodeType {
        NodeType::PathExpr
    }
}
