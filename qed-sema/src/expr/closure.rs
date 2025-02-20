use crate::{CheckedFunctionSignature, IdentId, NodeInfo, NodeType, StmtId, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedClosureNode {
    pub parameters: Vec<(IdentId, bool, TypeId)>,
    pub body: StmtId,
    pub return_type: Option<TypeId>,
    pub type_id: TypeId,
}

impl CheckedClosureNode {
    pub fn signature(&self) -> CheckedFunctionSignature {
        CheckedFunctionSignature {
            parameters: self
                .parameters
                .iter()
                .map(|(_, mutable, ty)| (mutable.clone(), ty.clone()))
                .collect(),
            return_type: self.return_type.clone(),
        }
    }
}

impl NodeInfo for CheckedClosureNode {
    fn node_type(&self) -> NodeType {
        NodeType::ClosureExpr
    }
}
