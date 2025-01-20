use std::collections::HashMap;

use enum_as_inner::EnumAsInner;
use strum::{EnumIs, EnumTryAs};

use crate::{AstVisitor, ExprId, IdentId, UncheckedType};

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum ValueNode<F: Clone> {
    Felt(F),
    Bool(F),
    Array(usize, Vec<ExprId>),
    Struct(IdentId, Vec<UncheckedType>, HashMap<IdentId, ExprId>),
}
