mod assignment;
mod block;
mod r#if;
mod r#return;
mod variable;
mod r#while;

pub use assignment::*;
pub use block::*;
pub use r#if::*;
pub use r#return::*;
pub use r#while::*;
pub use variable::*;

use crate::{AstVisitor, DefinitionNode, ExprNode};
use strum::EnumIs;

#[derive(Debug, Clone, PartialEq, EnumIs)]
pub enum StmtNode<F: Clone> {
    If(IfNode),
    While(WhileNode),
    Block(BlockNode),
    Assignment(AssignmentNode),
    Variable(VariableNode),
    Definition(DefinitionNode),
    Expression(ExprNode<F>),
    Return(ReturnNode),
}

impl<F: Clone> StmtNode<F> {
    pub fn accept_visitor<C, V: AstVisitor<F, C>>(&self, visitor: &mut V) -> V::StmtResult {
        visitor.visit_stmt(self)
    }
}
