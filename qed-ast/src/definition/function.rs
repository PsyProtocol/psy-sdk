use crate::{
    AttrNode, ExprId, IdentId, NodeInfo, NodeType, Qualifier, Span, TypeQualifier, UncheckedType,
    Visibility,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionNode {
    pub name: IdentId,
    pub parameters: Vec<FunctionParameter>,
    pub generic_parameters: Vec<IdentId>,
    pub body: Option<ExprId>,
    pub return_type: Option<UncheckedType>,
    pub qualifier: Qualifier,
    pub visibility: Visibility,
    pub attrs: Vec<AttrNode>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParameter {
    pub name: IdentId,
    pub qualifier: TypeQualifier,
    pub ty: UncheckedType,
    pub span: Span,
}

impl FunctionParameter {
    pub fn new(name: IdentId, qualifier: TypeQualifier, ty: UncheckedType, span: Span) -> Self {
        Self {
            name,
            qualifier,
            ty,
            span,
        }
    }
}

impl NodeInfo for FunctionNode {
    fn node_type(&self) -> NodeType {
        NodeType::FunctionDef
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    pub parameters: Vec<UncheckedType>,
    pub return_type: Option<UncheckedType>,
}
