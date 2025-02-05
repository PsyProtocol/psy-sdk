use enum_as_inner::EnumAsInner;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumAsInner)]
pub enum Visibility {
    Public,
    Private,
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Private
    }
}
