use crate::{arena::ExprId, visitor::AstVisitor, Type, VariableNode};

#[derive(Debug, Clone, PartialEq)]
pub struct CallNode {
    pub variable: VariableNode,
    pub generic_parameters: Vec<Type>,
    pub args: Vec<ExprId>,
}

impl CallNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::ExprResult {
        visitor.visit_call(self)
    }
}
