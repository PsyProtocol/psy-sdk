use crate::{AstVisitor, BlockNode, Case, ExprId, IfNode, NodeInfo, NodeType, StmtId};

#[derive(Debug, Clone, PartialEq)]
pub struct IfExprNode {
    pub if_branch: Case,
    pub elseif_branches: Vec<Case>,
    pub else_branch: Option<StmtId>,
}

impl NodeInfo for IfExprNode {
    fn node_type(&self) -> NodeType {
        NodeType::IfExpr
    }
}

impl From<IfNode> for IfExprNode {
    fn from(if_node: IfNode) -> Self {
        Self {
            if_branch: if_node.if_branch,
            elseif_branches: if_node.elseif_branch,
            else_branch: if_node.else_branch,
        }
    }
}
