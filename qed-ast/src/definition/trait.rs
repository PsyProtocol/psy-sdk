use crate::{AstVisitor, FunctionNode, IdentId};

#[derive(Clone, Debug, PartialEq)]
pub struct TraitNode {
    pub name: IdentId,
    pub generic_parameters: Vec<IdentId>,
    pub body: Vec<FunctionNode>,
}
