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

use crate::{visitor::AstVisitor, EnumNode, FunctionNode, ImplNode, StructNode};
use strum::EnumIs;

#[derive(Debug, Clone, PartialEq, EnumIs)]
pub enum StmtNode {
    If(IfNode),
    While(WhileNode),
    Block(BlockNode),
    Assignment(AssignmentNode),
    VarDecl(VarDeclNode),
    StructDecl(StructNode),
    EnumDecl(EnumNode),
    FunctionDecl(FunctionNode),
    Impl(ImplNode),
    Return(ReturnNode),
}

impl StmtNode {
    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::StmtResult {
        visitor.visit_stmt(self)
    }
}
