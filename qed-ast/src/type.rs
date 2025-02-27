use enum_as_inner::EnumAsInner;

use crate::{FunctionSignature, IdentId};

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum UncheckedType {
    Basic(IdentId),                       // u8, T
    Generic(IdentId, Vec<UncheckedType>), // HashMap<K, V>
    Array(Box<UncheckedType>, usize),     // [u8; 10]
    Tuple(Vec<UncheckedType>),
    FunctionSignature(Box<FunctionSignature>),
    Unknown,
}
