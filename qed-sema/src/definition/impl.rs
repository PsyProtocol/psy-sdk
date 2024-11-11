use qed_ast::IdentId;

use crate::{CheckedFunctionNode, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedImplNode {
    pub generic_parameters: Vec<TypeId>,
    pub ty: IdentId,
    pub body: Vec<CheckedFunctionNode>,
}
