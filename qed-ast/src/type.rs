use crate::arena::IdentId;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Basic(IdentId),              // u8, T
    Generic(IdentId, Vec<Type>), // HashMap<K, V>
    Array(Box<Type>, usize),     // [u8; 10]
    Tuple(Vec<Type>),
    // Option(Box<Type<'a>>),
    // Result(Box<Type<'a>>, Box<Type<'a>>),
}
