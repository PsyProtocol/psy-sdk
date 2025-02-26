use enum_as_inner::EnumAsInner;

use crate::{FunctionSignature, IdentId};

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum UncheckedType {
    Basic(IdentId),                       // u8, T
    Generic(IdentId, Vec<UncheckedType>), // HashMap<K, V>
    Array(Box<UncheckedType>, usize),     // [u8; 10]
    // Tuple(Vec<UncheckedType>),
    FunctionSignature(Box<FunctionSignature>),
    Unknown,
}

impl UncheckedType {
    pub fn name(&self) -> IdentId {
        match self {
            UncheckedType::Basic(ident_id) => ident_id.clone(),
            UncheckedType::Generic(ident_id, _) => ident_id.clone(),
            UncheckedType::Array(_, _) => IdentId::TYPE_ARRAY,
            UncheckedType::Unknown => IdentId::TYPE_UNKNOWN,
            _ => unreachable!(),
        }
    }
}
