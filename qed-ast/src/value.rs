use std::collections::HashMap;

use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;
use strum::{EnumIs, EnumTryAs};

use crate::{AstVisitor, ExprId, IdentId, UncheckedType};

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum ValueNode<F: Clone + From<u32>> {
    Felt(F),
    Bool(F),
    Array(usize, Vec<ExprId>),
    Struct(IdentId, Vec<UncheckedType>, IndexMap<IdentId, ExprId>),
}
