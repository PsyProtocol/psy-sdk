use crate::CheckedGenericParameter;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Constraint {
    constraints: Vec<CheckedGenericParameter>,
}
