use qed_ast::ExprId;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedStorageWriteNode {
    pub offset: ExprId,
    pub value: ExprId,
}
