use crate::{arena::IdentId, AstVisitor, BlockNode, Type};

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionNode {
    pub name: IdentId,
    pub parameters: Vec<(IdentId, Type)>,
    pub generic_parameters: Vec<IdentId>,
    pub body: BlockNode,
    pub return_type: Option<Type>,
}

impl FunctionNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::StmtResult {
        visitor.visit_function(self)
    }
}
