use enum_as_inner::EnumAsInner;

use crate::{ExprId, NodeInfo, NodeType, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct MatchNode {
    pub scrutinee: ExprId,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: ExprId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum MatchPattern {
    Value(ExprId, Span),
    PlaceHolder(Span),
}

impl MatchPattern {
    pub fn span(&self) -> Span {
        match self {
            MatchPattern::Value(_, span) => span.clone(),
            MatchPattern::PlaceHolder(span) => span.clone(),
        }
    }
}
impl NodeInfo for MatchNode {
    fn node_type(&self) -> NodeType {
        NodeType::MatchExpr
    }
}
