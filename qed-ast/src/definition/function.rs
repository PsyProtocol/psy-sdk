use crate::{AstVisitor, BlockNode, IdentId, StmtId, UncheckedType};

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionNode {
    pub name: IdentId,
    pub parameters: Vec<(IdentId, bool, UncheckedType)>,
    pub generic_parameters: Vec<IdentId>,
    pub body: Option<StmtId>,
    pub return_type: Option<UncheckedType>,
}
