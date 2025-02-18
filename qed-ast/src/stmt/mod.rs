mod assert;
mod assignment;
mod block;
mod r#return;
mod storage;
mod variable;
mod r#while;

pub use assert::*;
pub use assignment::*;
pub use block::*;
use enum_as_inner::EnumAsInner;
pub use r#return::*;
pub use r#while::*;
use std::fmt::Display;
pub use storage::*;
pub use variable::*;

use crate::{AstVisitor, DefId, DefinitionNode, ExprId, ExprNode, NodeInfo, NodeType, UsePath};
use strum::EnumTryAs;

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum StmtNode {
    While(WhileNode),
    Block(BlockNode),
    Assignment(AssignmentNode),
    Variable(VariableNode),
    Definition(DefId),
    Expression(ExprId),
    Return(ReturnNode),
    Storage(StorageWriteNode),
    Assert(AssertNode),
    AssertEq(AssertEqNode),
    Use(UsePath),
}

impl NodeInfo for StmtNode {
    fn node_type(&self) -> NodeType {
        match self {
            StmtNode::While(node) => node.node_type(),
            StmtNode::Block(node) => node.node_type(),
            StmtNode::Assignment(node) => node.node_type(),
            StmtNode::Variable(node) => node.node_type(),
            StmtNode::Definition(_) => NodeType::DefinitionStmt,
            StmtNode::Expression(_) => NodeType::ExpressionStmt,
            StmtNode::Return(node) => node.node_type(),
            StmtNode::Storage(node) => node.node_type(),
            StmtNode::Assert(node) => node.node_type(),
            StmtNode::AssertEq(node) => node.node_type(),
            StmtNode::Use(_) => NodeType::UseStmt,
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
            StmtNode::While(_) => write!(f, "While"),
            StmtNode::Block(_) => write!(f, "Block"),
            StmtNode::Assignment(_) => write!(f, "Assignment"),
            StmtNode::Variable(_) => write!(f, "Variable"),
            StmtNode::Definition(_) => write!(f, "Definition"),
            StmtNode::Expression(_) => write!(f, "Expression"),
            StmtNode::Return(_) => write!(f, "Return"),
            StmtNode::Storage(_) => write!(f, "Storage::Write"),
            StmtNode::Assert(_) => write!(f, "Assert"),
            StmtNode::AssertEq(_) => write!(f, "AssertEq"),
            StmtNode::Use(_) => write!(f, "Use"),
        }
    }
}

impl From<ExprId> for StmtNode {
    fn from(value: ExprId) -> Self {
        Self::Expression(value)
    }
}

impl From<DefId> for StmtNode {
    fn from(value: DefId) -> Self {
        Self::Definition(value)
    }
}
