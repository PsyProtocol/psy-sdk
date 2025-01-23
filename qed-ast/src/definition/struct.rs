use std::collections::HashMap;

use crate::{AstVisitor, AttrNode, FunctionNode, IdentId, UncheckedType, ValueNode};

#[derive(Clone, Debug, PartialEq)]
pub struct StructNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub fields: Vec<(IdentId, UncheckedType)>,
    pub attrs: Vec<AttrNode>,
}
