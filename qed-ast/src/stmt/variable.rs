use crate::{
    arena::{ExprId, IdentId},
    r#type::Type,
    visitor::AstVisitor,
};

#[derive(Debug, Clone, PartialEq)]
pub struct VarDeclNode {
    pub name: IdentId,
    pub ty: Type,
    pub mutable: bool,
    pub value: ExprId,
}

impl VarDeclNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::StmtResult {
        visitor.visit_var_decl(self)
    }
}
