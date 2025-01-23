mod assignment;
mod block;
mod r#if;
mod r#return;
mod storage;
mod variable;
mod r#while;

pub use assignment::*;
pub use block::*;
pub use r#if::*;
pub use r#return::*;
pub use r#while::*;
pub use storage::*;
pub use variable::*;

use qed_ast::StmtNode;
use strum::EnumIs;

use crate::{CheckedDefinitionNode, CheckedExprNode, TypeId};

#[derive(Debug, Clone, PartialEq, EnumIs)]
pub enum CheckedStmtNode<F> {
    If(CheckedIfNode),
    While(CheckedWhileNode),
    Block(CheckedBlockNode),
    Assignment(CheckedAssignmentNode),
    Variable(CheckedVariableNode),
    Definition(CheckedDefinitionNode),
    Expression(CheckedExprNode<F>),
    Storage(CheckedStorageWriteNode),
    Return(CheckedReturnNode),
}

impl<F> From<CheckedExprNode<F>> for CheckedStmtNode<F> {
    fn from(value: CheckedExprNode<F>) -> Self {
        Self::Expression(value)
    }
}
