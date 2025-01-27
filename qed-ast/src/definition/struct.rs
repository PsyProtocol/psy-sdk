use std::collections::HashMap;

use crate::{AstVisitor, AttrNode, FunctionNode, IdentId, NodeType, UncheckedType, ValueNode};

#[derive(Clone, Debug, PartialEq)]
pub struct StructNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub fields: Vec<(IdentId, UncheckedType, bool)>,
    pub attrs: Vec<AttrNode>,
    pub is_pub: bool,
}

impl StructNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::StructDef
    }
}
