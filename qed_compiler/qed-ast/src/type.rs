use std::fmt::Display;

use enum_as_inner::EnumAsInner;

use crate::{ConstValue, FunctionSignature, Identifier, Location, PathNode};

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum UncheckedType {
    Const(ConstValue, Location),
    Basic(Box<PathNode>),                              // u8, T
    Generic(Identifier, Vec<UncheckedType>, Location), // IndexMap<K, V>
    Array(Box<UncheckedType>, u32, Location),          // [u8; 10]
    Tuple(Vec<UncheckedType>, Location),
    FunctionSignature(Box<FunctionSignature>, Location),
    Unknown,
}

impl UncheckedType {
    pub fn location(&self) -> Location {
        match self {
            UncheckedType::Const(_, location) => *location,
            UncheckedType::Basic(ty) => ty.location,
            UncheckedType::Generic(_, _, location) => *location,
            UncheckedType::Array(_, _, location) => *location,
            UncheckedType::Tuple(_unchecked_types, location) => *location,
            UncheckedType::FunctionSignature(_function_signature, location) => *location,
            UncheckedType::Unknown => Location::default(),
        }
    }
}
