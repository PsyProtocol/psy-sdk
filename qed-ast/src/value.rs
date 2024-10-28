use std::collections::HashMap;

use strum::{EnumIs, EnumTryAs};

use crate::{arena::IdentId, visitor::AstVisitor};

#[derive(Clone, Debug, PartialEq, EnumIs, EnumTryAs)]
pub enum ValueNode<F> {
    Felt(F),
    Array(Vec<ValueNode<F>>, usize),
    Struct(HashMap<IdentId, ValueNode<F>>),
}

impl<F> ValueNode<F> {
    pub fn accept_visitor<V: AstVisitor<F>>(&mut self, visitor: &mut V) {
        visitor.visit_value(self);
    }
}
