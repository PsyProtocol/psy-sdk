mod assignment;
mod block;
mod r#if;
mod intrinsic;
mod r#return;
mod variable;
mod r#while;

pub use assignment::*;
pub use block::*;
use enum_as_inner::EnumAsInner;
pub use intrinsic::*;
pub use r#if::*;
pub use r#return::*;
pub use r#while::*;
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
    Return(CheckedReturnNode),
    Intrinsic(CheckedIntrinsicStmtNode),
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
            Self::Return(node) => node.node_type(),
            Self::Intrinsic(node) => node.node_type(),
        }
    }

    fn as_expression(&self) -> Option<ExprId> {
        match self {
            Self::Expression(expr) => Some(*expr),
            _ => None,
        }
    }

    fn as_definition(&self) -> Option<DefId> {
        match self {
            Self::Definition(def) => Some(*def),
            _ => None,
        }
    }
}

impl From<ExprId> for CheckedStmtNode {
    fn from(value: ExprId) -> Self {
        Self::Expression(value)
    }
}

impl From<DefId> for CheckedStmtNode {
    fn from(value: DefId) -> Self {
        Self::Definition(value)
    }
}

impl<F> From<CheckedExprNode<F>> for CheckedStmtNode {
    fn from(value: CheckedExprNode<F>) -> Self {
        todo!()
    }
}

impl From<CheckedDefinitionNode> for CheckedStmtNode {
    fn from(value: CheckedDefinitionNode) -> Self {
        todo!()
    }
}
