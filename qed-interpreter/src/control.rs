use enum_as_inner::EnumAsInner;
use qed_ast::IdentId;
use std::ops::{ControlFlow, FromResidual, Try};

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum ControlState<T> {
    Normal,
    Return(T),
}

impl<T> Try for ControlState<T> {
    type Output = ();
    type Residual = Self;

    fn from_output(_: Self::Output) -> Self {
        ControlState::Normal
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            ControlState::Normal => ControlFlow::Continue(()),
            other => ControlFlow::Break(other),
        }
    }
}

impl<T> FromResidual for ControlState<T> {
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        residual
    }
}
