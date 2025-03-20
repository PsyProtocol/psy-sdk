use crate::{
    AttrNode, ExprId, GenericParameter, IdentId, Location, NodeInfo, NodeType, Qualifier,
    TypeQualifier, UncheckedType, Visibility,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionNode {
    pub name: IdentId,
    pub parameters: Vec<FunctionParameter>,
    pub generic_parameters: Vec<GenericParameter>,
    pub body: Option<ExprId>,
    pub return_type: Option<UncheckedType>,
    pub qualifier: Qualifier,
    pub visibility: Visibility,
    pub attrs: Vec<AttrNode>,
    pub location: Location,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParameter {
    pub name: IdentId,
    pub qualifier: TypeQualifier,
    pub ty: UncheckedType,
    pub location: Location,
}

impl FunctionParameter {
    pub fn new(
        name: IdentId,
        qualifier: TypeQualifier,
        ty: UncheckedType,
        location: Location,
    ) -> Self {
        Self {
            name,
            qualifier,
            ty,
            location,
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
