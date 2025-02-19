use enum_as_inner::EnumAsInner;
use std::{
    convert::Infallible,
    ops::{ControlFlow, FromResidual, Try},
};

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
impl<T, E1, E2> FromResidual<Result<Infallible, E1>> for ControlState<Result<T, E2>>
where
    E1: Into<E2>,
{
    fn from_residual(residual: Result<Infallible, E1>) -> Self {
        match residual {
            Err(e) => ControlState::Return(Err(e.into())),
            Ok(infallible) => match infallible {},
        }
    }
}
impl<T> ControlState<T> {
    pub fn unwrap(self) -> Option<T> {
        match self {
            ControlState::Return(value) => Some(value),
            ControlState::Normal => None,
        }
    }
}
