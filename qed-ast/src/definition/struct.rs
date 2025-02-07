use std::collections::HashMap;

use indexmap::IndexMap;

use crate::{
    AstVisitor, AttrNode, FunctionNode, IdentId, NodeInfo, NodeType, UncheckedType, ValueNode,
    Visibility,
};

#[derive(Clone, Debug, PartialEq)]
pub struct StructNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub fields: IndexMap<IdentId, (UncheckedType, Visibility)>,
    pub attrs: Vec<AttrNode>,
    pub visibility: Visibility,
}

impl NodeInfo for StructNode {
    fn node_type(&self) -> NodeType {
        NodeType::StructDef
    }
}
