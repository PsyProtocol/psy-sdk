use enum_as_inner::EnumAsInner;

use crate::{FunctionSignature, IdentId, Identifier, Location};

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum UncheckedType {
    Basic(Identifier),                                 // u8, T
    Generic(Identifier, Vec<UncheckedType>, Location), // IndexMap<K, V>
    Array(Box<UncheckedType>, usize, Location),        // [u8; 10]
    Tuple(Vec<UncheckedType>, Location),
    FunctionSignature(Box<FunctionSignature>, Location),
    Unknown,
}

impl UncheckedType {
    pub fn name(&self) -> IdentId {
        match self {
            UncheckedType::Basic(ident_id) => ident_id.id,
            UncheckedType::Generic(ident_id, _, _) => ident_id.id,
            UncheckedType::Array(_, _, _) => IdentId::TYPE_ARRAY,
            UncheckedType::Unknown => IdentId::TYPE_UNKNOWN,
            _ => unreachable!(),
        }
    }
}
