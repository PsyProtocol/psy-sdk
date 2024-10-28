use crate::{arena::IdentId, FunctionNode, Type};

#[derive(Debug, Clone, PartialEq)]
pub struct ImplNode {
    pub generic_parameters: Vec<IdentId>,
    pub ty: Type,
    pub body: Vec<FunctionNode>,
}
