use crate::{
    AstVisitor, {ExprId, IdentId},
};

#[derive(Clone, Debug, PartialEq)]
pub struct PathNode(pub IdentId);
