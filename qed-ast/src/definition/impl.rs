use crate::{AstVisitor, DefId, FunctionNode, IdentId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct ImplNode {
    pub generic_parameters: Vec<IdentId>,
    pub trait_name: Option<IdentId>,
    pub ty: IdentId,
    pub body: Vec<DefId>,
}
