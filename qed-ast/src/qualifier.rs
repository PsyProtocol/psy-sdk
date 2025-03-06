use std::fmt::Display;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct Qualifier {
    pub is_extern: bool,
    pub is_const: bool,
}

impl Default for Qualifier {
    fn default() -> Self {
        Self {
            is_extern: false,
            is_const: false,
        }
    }
}

impl Display for Qualifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_extern {
            write!(f, "extern ")?;
        }
        if self.is_const {
            write!(f, "const ")?;
        }
        Ok(())
    }
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

impl Display for TypeQualifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_mutable {
            write!(f, "mut ")?;
        }
        Ok(())
    }
}

impl TypeQualifier {
    pub fn new(is_mutable: bool) -> TypeQualifier {
        TypeQualifier { is_mutable }
    }
}
