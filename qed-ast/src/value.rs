use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;

use crate::{ExprId, Identifier, Location, NodeInfo, NodeType, UncheckedType};

#[derive(Clone, Debug, PartialEq, EnumAsInner)]
pub enum ValueNode<F: Clone + From<u32>> {
    Felt(F, Location),
    Bool(F, Location),
    U32(F, Location),
    Array(usize, Vec<ExprId>, Location),
    Struct(
        Identifier,
        Vec<UncheckedType>,
        IndexMap<Identifier, ExprId>,
        Location,
    ),
}

impl<F: Clone + From<u32>> NodeInfo for ValueNode<F> {
    fn node_type(&self) -> NodeType {
        NodeType::ValueExpr
    }
}
