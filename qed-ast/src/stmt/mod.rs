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
use std::fmt::Display;
pub use storage::*;
pub use variable::*;

use crate::{AstVisitor, DefId, DefinitionNode, ExprId, ExprNode, NodeInfo, NodeType};
use strum::EnumTryAs;

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum StmtNode {
    If(IfNode),
    While(WhileNode),
    Block(BlockNode),
    Assignment(AssignmentNode),
    Variable(VariableNode),
    Definition(DefId),
    Expression(ExprId),
    Return(ReturnNode),
    Storage(StorageWriteNode),
}

impl NodeInfo for StmtNode {
    fn node_type(&self) -> NodeType {
        match self {
            StmtNode::If(node) => node.node_type(),
            StmtNode::While(node) => node.node_type(),
            StmtNode::Block(node) => node.node_type(),
            StmtNode::Assignment(node) => node.node_type(),
            StmtNode::Variable(node) => node.node_type(),
            StmtNode::Definition(_) => NodeType::DefinitionStmt,
            StmtNode::Expression(_) => NodeType::ExpressionStmt,
            StmtNode::Return(node) => node.node_type(),
            StmtNode::Storage(node) => node.node_type(),
        }
    }

    fn as_expression(&self) -> Option<ExprId> {
        match self {
            StmtNode::Expression(expr) => Some(*expr),
            _ => None,
        }
    }

    fn as_definition(&self) -> Option<DefId> {
        match self {
            StmtNode::Definition(def) => Some(*def),
            _ => None,
        }
    }
}

impl Display for StmtNode {
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
            StmtNode::Storage(_) => write!(f, "Storage::Write"),
        }
    }
}
