mod assignment;
mod block;
mod r#if;
mod r#return;
mod variable;
mod r#while;

pub use assignment::*;
pub use block::*;
use enum_as_inner::EnumAsInner;
pub use r#if::*;
pub use r#return::*;
pub use r#while::*;
pub use variable::*;

use crate::{AstVisitor, DefId, DefinitionNode, ExprId, ExprNode, NodeType};
use strum::{EnumIs, EnumTryAs};

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
}

impl StmtNode {
    pub fn node_type(&self) -> NodeType {
        match self {
            StmtNode::If(_) => NodeType::IfStmt,
            StmtNode::While(_) => NodeType::WhileStmt,
            StmtNode::Block(_) => NodeType::BlockStmt,
            StmtNode::Assignment(_) => NodeType::AssignmentStmt,
            StmtNode::Variable(_) => NodeType::VariableStmt,
            StmtNode::Definition(_) => NodeType::DefinitionStmt,
            StmtNode::Expression(_) => NodeType::ExpressionStmt,
            StmtNode::Return(_) => NodeType::ReturnStmt,
        }
    }
}
