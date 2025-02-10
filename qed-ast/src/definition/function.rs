use crate::{
    AstVisitor, AttrNode, BlockNode, IdentId, NodeInfo, NodeType, StmtId, UncheckedType, Visibility,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionNode {
    pub name: IdentId,
    pub parameters: Vec<(IdentId, bool, UncheckedType)>,
    pub generic_parameters: Vec<IdentId>,
    pub body: Option<StmtId>,
    pub return_type: Option<UncheckedType>,
    pub is_extern: bool,
    pub visibility: Visibility,
    pub attrs: Vec<AttrNode>,
}

impl NodeInfo for FunctionNode {
    fn node_type(&self) -> NodeType {
        NodeType::FunctionDef
    }
}
