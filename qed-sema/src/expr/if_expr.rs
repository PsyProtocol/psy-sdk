use crate::{CheckedCase, TypeId};
use qed_ast::{ExprId, NodeInfo, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedIfExprNode {
    pub if_branch: CheckedCase,
    pub elseif_branch: Vec<CheckedCase>,
    pub else_branch: Option<StmtId>,
    pub type_id: TypeId,
}

impl NodeInfo for CheckedIfExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::IfExpr
    }
}
