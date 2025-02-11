use std::collections::HashMap;

use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;
use strum::EnumTryAs;

use crate::{AstVisitor, ExprId, IdentId, NodeInfo, NodeType, UncheckedType};

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum ValueNode<F: Clone + From<u32>> {
    Felt(F),
    Bool(F),
    String(String),
    Array(usize, Vec<ExprId>),
    Struct(IdentId, Vec<UncheckedType>, IndexMap<IdentId, ExprId>),
}

impl<F: Clone + From<u32>> NodeInfo for ValueNode<F> {
    fn node_type(&self) -> NodeType {
        NodeType::ValueExpr
    }
}
