use crate::{AstVisitor, ExprId, IdentId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct VariableNode {
    pub name: IdentId,
    pub ty: UncheckedType,
    pub mutable: bool,
    pub cnst: bool,
    pub value: ExprId,
}
