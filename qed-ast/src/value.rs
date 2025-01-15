use std::collections::HashMap;

use strum::{EnumIs, EnumTryAs};

use crate::{AstVisitor, ExprId, IdentId, UncheckedType};

#[derive(Clone, Debug, PartialEq, EnumIs, EnumTryAs)]
pub enum ValueNode<F: Clone> {
    Felt(F),
    Bool(F),
    Array(usize, Vec<ExprId>),
    Struct(IdentId, Vec<UncheckedType>, HashMap<IdentId, ExprId>),
}

impl<F: Clone> ValueNode<F> {
    pub fn accept_visitor<C, V: AstVisitor<F, C>>(&self, visitor: &mut V, ctx: &mut V::Context) {
        visitor.visit_value(self, ctx);
    }
}
