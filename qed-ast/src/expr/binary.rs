use crate::{arena::ExprId, visitor::AstVisitor};

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,

    Mod,

    BitShr,
    BitShl,

    BitAnd,
    BitOr,
    BitXor,

    And,
    Or,

    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryNode {
    pub lhs: ExprId,
    pub operator: BinaryOperator,
    pub rhs: ExprId,
}

impl BinaryNode {
    pub fn new(lhs: ExprId, operator: BinaryOperator, rhs: ExprId) -> Self {
        Self { lhs, operator, rhs }
    }

    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::ExprResult {
        visitor.visit_binary(self)
    }
}
