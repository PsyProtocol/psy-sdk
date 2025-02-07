use std::collections::HashMap;

use indexmap::IndexMap;

use crate::{AstVisitor, FunctionNode, IdentId, NodeInfo, NodeType, UncheckedType, Visibility};

#[derive(Clone, Debug, PartialEq)]
pub enum EnumVariant {
    Basic(IdentId),
    Tuple(IdentId, Vec<UncheckedType>),
    Struct(IdentId, IndexMap<IdentId, (UncheckedType, Visibility)>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub variants: Vec<EnumVariant>,
    pub visibility: Visibility,
}

impl NodeInfo for EnumNode {
    fn node_type(&self) -> NodeType {
        NodeType::EnumDef
    }
}
