use enum_as_inner::EnumAsInner;

use crate::{FunctionSignature, IdentId, Span};

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum UncheckedType {
    Basic(IdentId, Span),                       // u8, T
    Generic(IdentId, Vec<UncheckedType>, Span), // HashMap<K, V>
    Array(Box<UncheckedType>, usize, Span),     // [u8; 10]
    Tuple(Vec<UncheckedType>, Span),
    FunctionSignature(Box<FunctionSignature>, Span),
    Unknown,
}

impl UncheckedType {
    pub fn name(&self) -> IdentId {
        match self {
            UncheckedType::Basic(ident_id, _) => ident_id.clone(),
            UncheckedType::Generic(ident_id, _, _) => ident_id.clone(),
            UncheckedType::Array(_, _, _) => IdentId::TYPE_ARRAY,
            UncheckedType::Unknown => IdentId::TYPE_UNKNOWN,
            _ => unreachable!(),
        }
    }
}
