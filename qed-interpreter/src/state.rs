use qed_ast::IdentId;
use std::ops::{ControlFlow, FromResidual, Try};
use strum::EnumIs;

#[derive(Clone, Debug, PartialEq, EnumIs)]
pub enum ControlState {
    Normal,
    Break(IdentId),
    Continue(IdentId),
}

impl Try for ControlState {
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

impl FromResidual for ControlState {
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        residual
    }
}
