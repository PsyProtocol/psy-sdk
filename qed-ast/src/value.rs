use enum_as_inner::EnumAsInner;
use indexmap::IndexMap;
use std::fmt::{Display, Formatter};

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

impl<F: Clone + From<u32>> Display for ValueNode<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueNode::Felt(_, _) => write!(f, "Felt"),
            ValueNode::Bool(_, _) => write!(f, "Bool"),
            ValueNode::U32(_, _) => write!(f, "U32"),
            ValueNode::Array(_, expr_ids, _) => {
                write!(f, "Array(")?;
                for (i, value) in expr_ids.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", value)?;
                }
                write!(f, ")")
            }
            ValueNode::Struct(name, _, fields, _) => {
                write!(f, "Struct {:?} {{ ", name)?;
                for (i, (field_name, field_value)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}: {:?}", field_name, field_value)?;
                }
                write!(f, " }}")
            }
        }
    }
}
