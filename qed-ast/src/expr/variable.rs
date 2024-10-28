use crate::arena::{ExprId, IdentId};

#[derive(Clone, Debug, PartialEq)]
pub enum PathSegment {
    IndexAccess(ExprId),
    MemberAccess(IdentId),
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableNode {
    pub name: IdentId,
    pub path: Vec<PathSegment>,
}
