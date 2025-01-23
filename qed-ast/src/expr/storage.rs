use crate::ExprId;

#[derive(Clone, Debug, PartialEq)]
pub struct StorageReadNode {
    pub offset: ExprId,
}
