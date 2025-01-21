use qed_ast::IdentId;

use crate::{CheckedFunctionNode, ScopeId, TypeId};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedTraitNode {
    pub name: IdentId,
    pub generic_parameters: Vec<TypeId>,
    pub body: Vec<CheckedFunctionNode>,
    pub implementors: Vec<TypeId>,
    pub scope_id: ScopeId,
}

impl CheckedTraitNode {
    pub fn add_implementor(&mut self, implementor: TypeId) {
        self.implementors.push(implementor);
    }
}
