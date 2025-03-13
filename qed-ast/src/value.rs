use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;

use crate::{ExprId, IdentId, NodeInfo, NodeType, Span, UncheckedType};

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum ValueNode<F: Clone + From<u32>> {
    Felt(F, Span),
    Bool(F, Span),
    U32(F, Span),
    Array(usize, Vec<ExprId>, Span),
    Struct(IdentId, Vec<UncheckedType>, IndexMap<IdentId, ExprId>, Span),
}

impl<F: Clone + From<u32>> NodeInfo for ValueNode<F> {
    fn node_type(&self) -> NodeType {
        NodeType::ValueExpr
    }
}
