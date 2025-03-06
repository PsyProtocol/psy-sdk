use qed_ast::{
    AttrNode, ExprId, IdentId, NodeInfo, NodeType, Qualifier, TypeQualifier, Visibility,
};

use crate::{ScopeId, TypeId, UNKOWN_TYPE};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedFunctionNode {
    pub name: IdentId,
    pub parameters: Vec<(IdentId, TypeQualifier, TypeId)>,
    pub generic_parameters: Vec<TypeId>,
    pub body: Option<ExprId>,
    pub qualifier: Qualifier,
    pub return_type: TypeId,
    pub scope_id: ScopeId,
    pub visibility: Visibility,
    pub attrs: Vec<AttrNode>,
}

impl Default for CheckedFunctionNode {
    fn default() -> Self {
        Self {
            name: IdentId(0),
            parameters: Vec::new(),
            generic_parameters: Vec::new(),
            body: None,
            qualifier: Default::default(),
            return_type: TypeId(0),
            scope_id: ScopeId(0),
            visibility: Visibility::Public,
            attrs: Vec::new(),
        }
    }
}

impl CheckedFunctionNode {
    pub fn signature(&self) -> CheckedFunctionSignature {
        CheckedFunctionSignature {
            parameters: self
                .parameters
                .iter()
                .map(|(_, _, ty)| ty.clone())
                .collect(),
            return_type: self.return_type,
        }
    }

    pub fn trait_impl_signature(&self, implementor_type_id: TypeId) -> CheckedFunctionSignature {
        CheckedFunctionSignature {
            parameters: self
                .parameters
                .iter()
                .map(|(_, _, ty)| {
                    if ty == &UNKOWN_TYPE {
                        implementor_type_id.clone()
                    } else {
                        ty.clone()
                    }
                })
                .collect(),
            return_type: if self.return_type == UNKOWN_TYPE {
                implementor_type_id
            } else {
                self.return_type.clone()
            },
        }
    }
}

impl NodeInfo for CheckedFunctionNode {
    fn node_type(&self) -> NodeType {
        NodeType::FunctionDef
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedFunctionSignature {
    pub parameters: Vec<TypeId>,
    pub return_type: TypeId,
}
