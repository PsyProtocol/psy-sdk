use crate::{AstVisitor, ExprId};

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnNode(pub Option<ExprId>);
