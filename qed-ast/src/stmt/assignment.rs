use crate::{arena::ExprId, visitor::AstVisitor, VariableNode};

#[derive(Clone, Debug, PartialEq)]
pub enum AssignmentOperator {
    Eq,

    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,

    ModAssign,

    BitAndAssign,
    BitOrAssign,
    BitXorAssign,

    BitShlAssign,
    BitShrAssign,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssignmentNode {
    pub variable: VariableNode,
    pub operator: AssignmentOperator,
    pub value: ExprId,
}

impl<'a> AssignmentNode {
    pub fn new(variable: VariableNode, operator: AssignmentOperator, value: ExprId) -> Self {
        Self {
            variable,
            operator,
            value,
        }
    }

    pub fn accept_visitor<F, V: AstVisitor<F>>(&mut self, visitor: &mut V) -> V::StmtResult {
        visitor.visit_assignment(self)
    }
}
