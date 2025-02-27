use crate::{
    AttrNode, IdentId, NodeInfo, NodeType, Qualifier, StmtId, TypeQualifier, UncheckedType,
    Visibility,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionNode {
    pub name: IdentId,
    pub parameters: Vec<(IdentId, TypeQualifier, UncheckedType)>,
    pub generic_parameters: Vec<IdentId>,
    pub body: Option<StmtId>,
    pub return_type: Option<UncheckedType>,
    pub qualifier: Qualifier,
    pub visibility: Visibility,
    pub attrs: Vec<AttrNode>,
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
