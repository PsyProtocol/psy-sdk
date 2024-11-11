use qed_ast::StmtId;

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedBlockNode {
    pub stmts: Vec<StmtId>,
}
