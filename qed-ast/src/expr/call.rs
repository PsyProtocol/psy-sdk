use crate::{AstVisitor, ExprId, PathNode, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct CallNode {
    pub variable: ExprId,
    pub receiver: Option<ExprId>,
    pub generic_parameters: Vec<UncheckedType>,
    pub args: Vec<ExprId>,
}
