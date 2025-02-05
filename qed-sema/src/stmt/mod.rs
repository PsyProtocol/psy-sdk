mod assignment;
mod block;
mod r#if;
mod r#return;
mod storage;
mod variable;
mod r#while;

pub use assignment::*;
pub use block::*;
use enum_as_inner::EnumAsInner;
pub use r#if::*;
pub use r#return::*;
pub use r#while::*;
pub use storage::*;
pub use variable::*;

use qed_ast::{NodeInfo, NodeType, StmtNode};

use crate::{CheckedDefinitionNode, CheckedExprNode, TypeId};

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
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

impl<F> NodeInfo for CheckedStmtNode<F> {
    fn node_type(&self) -> NodeType {
        match self {
            Self::If(node) => node.node_type(),
            Self::While(node) => node.node_type(),
            Self::Block(node) => node.node_type(),
            Self::Assignment(node) => node.node_type(),
            Self::Variable(node) => node.node_type(),
            Self::Definition(node) => node.node_type(),
            Self::Expression(node) => node.node_type(),
            Self::Storage(node) => node.node_type(),
            Self::Return(node) => node.node_type(),
        }
    }
}

impl<F> From<CheckedExprNode<F>> for CheckedStmtNode<F> {
    fn from(value: CheckedExprNode<F>) -> Self {
        Self::Expression(value)
    }
}
