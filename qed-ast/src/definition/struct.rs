use std::collections::HashMap;

use crate::{
    AstVisitor, AttrNode, FunctionNode, IdentId, NodeType, UncheckedType, ValueNode, Visibility,
};

#[derive(Clone, Debug, PartialEq)]
pub struct StructNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub fields: Vec<(IdentId, UncheckedType, Visibility)>,
    pub attrs: Vec<AttrNode>,
    pub visibility: Visibility,
}

impl StructNode {
    pub fn node_type(&self) -> NodeType {
        NodeType::StructDef
    }
}
