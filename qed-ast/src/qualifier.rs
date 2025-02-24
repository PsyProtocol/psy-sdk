#[derive(Copy, Debug, Clone, PartialEq)]
pub struct Qualifier {
    pub is_extern: bool,
    pub is_const: bool,
}

impl Qualifier {
    pub fn new(is_extern: bool, is_const: bool) -> Self {
        Self {
            is_extern,
            is_const,
        }
    }
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct TypeQualifier {
    pub is_mutable: bool,
}

impl TypeQualifier {
    pub fn new(is_mutable: bool) -> TypeQualifier {
        TypeQualifier { is_mutable }
    }
}
