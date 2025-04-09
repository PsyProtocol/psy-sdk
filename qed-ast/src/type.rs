use enum_as_inner::EnumAsInner;

use crate::{FunctionSignature, Identifier, Location, PathNode};

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum UncheckedType {
    Basic(Box<PathNode>),                              // u8, T
    Generic(Identifier, Vec<UncheckedType>, Location), // IndexMap<K, V>
    Array(Box<UncheckedType>, usize, Location),        // [u8; 10]
    Tuple(Vec<UncheckedType>, Location),
    FunctionSignature(Box<FunctionSignature>, Location),
    TraitCast(Box<UncheckedType>, Box<UncheckedType>, Location), // Type as Trait
    Unknown,
}

impl UncheckedType {
    pub fn location(&self) -> Location {
        match self {
            UncheckedType::Basic(ty) => ty.location,
            UncheckedType::Generic(_, _, location) => *location,
            UncheckedType::Array(_, _, location) => *location,
            UncheckedType::Tuple(_unchecked_types, location) => *location,
            UncheckedType::FunctionSignature(_function_signature, location) => *location,
            UncheckedType::TraitCast(_, _, location) => *location,
            UncheckedType::Unknown => Location::default(),
        }
    }
}
