mod r#enum;
mod function;
mod r#impl;
mod r#struct;

pub use function::*;
pub use r#enum::*;
pub use r#impl::*;
pub use r#struct::*;
use strum::EnumIs;

#[derive(Debug, Clone, PartialEq, EnumIs)]
pub enum CheckedDefinitionNode {
    Function(CheckedFunctionNode),
    Struct(CheckedStructNode),
    Enum(CheckedEnumNode),
    Impl(CheckedImplNode),
}
