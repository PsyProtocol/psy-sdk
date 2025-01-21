mod array;
mod r#enum;
mod function;
mod r#impl;
mod r#struct;
mod r#trait;

pub use array::*;
pub use function::*;
pub use r#enum::*;
pub use r#impl::*;
pub use r#struct::*;
pub use r#trait::*;

use strum::EnumIs;

#[derive(Debug, Clone, PartialEq, EnumIs)]
pub enum CheckedDefinitionNode {
    Function(CheckedFunctionNode),
    Struct(CheckedStructNode),
    Enum(CheckedEnumNode),
    Impl(CheckedImplNode),
    Trait(CheckedTraitNode),
}
