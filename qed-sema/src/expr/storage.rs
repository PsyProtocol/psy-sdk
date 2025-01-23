use qed_ast::ExprId;

use crate::TypeId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStorageReadNode {
    pub offset: ExprId,
    pub type_id: TypeId,
}
