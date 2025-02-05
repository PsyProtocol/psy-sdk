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

use qed_ast::{DefId, ExprId, NodeInfo, NodeType, StmtNode};

use crate::{CheckedDefinitionNode, CheckedExprNode, TypeId};

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum CheckedStmtNode {
    If(CheckedIfNode),
    While(CheckedWhileNode),
    Block(CheckedBlockNode),
    Assignment(CheckedAssignmentNode),
    Variable(CheckedVariableNode),
    Definition(DefId),
    Expression(ExprId),
    Storage(CheckedStorageWriteNode),
    Return(CheckedReturnNode),
}

impl NodeInfo for CheckedStmtNode {
    fn node_type(&self) -> NodeType {
        match self {
            Self::If(node) => node.node_type(),
            Self::While(node) => node.node_type(),
            Self::Block(node) => node.node_type(),
            Self::Assignment(node) => node.node_type(),
            Self::Variable(node) => node.node_type(),
            Self::Definition(node) => NodeType::DefinitionStmt,
            Self::Expression(node) => NodeType::ExpressionStmt,
            Self::Storage(node) => node.node_type(),
            Self::Return(node) => node.node_type(),
        }
    }
}
