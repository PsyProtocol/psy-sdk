mod assignment;
mod block;
mod r#if;
mod r#return;
mod variable;
mod r#while;

use std::fmt::Display;
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

impl <F: Clone>Display for StmtNode<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StmtNode::If(_) => write!(f, "If"),
            StmtNode::While(_) => write!(f, "While"),
            StmtNode::Block(_) => write!(f, "Block"),
            StmtNode::Assignment(_) => write!(f, "Assignment"),
            StmtNode::Variable(_) => write!(f, "Variable"),
            StmtNode::Definition(_) => write!(f, "Definition"),
            StmtNode::Expression(_) => write!(f, "Expression"),
            StmtNode::Return(_) => write!(f, "Return"),


        }
    }
}