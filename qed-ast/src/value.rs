use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;

use crate::{ExprId, IdentId, NodeInfo, NodeType, UncheckedType};

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum ValueNode<F: Clone + From<u32>> {
    Felt(F),
    Bool(F),
    U32(F),
    Array(usize, Vec<ExprId>),
    Struct(IdentId, Vec<UncheckedType>, IndexMap<IdentId, ExprId>),
}

impl<F: Clone + From<u32>> NodeInfo for ValueNode<F> {
    fn node_type(&self) -> NodeType {
        NodeType::ValueExpr
    }
}
