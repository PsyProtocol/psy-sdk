use std::collections::HashMap;

use qed_common::{Arena, FileId, Graph};

use crate::{AstVisitor, ExprId, ExprNode, IdentId, Interner, RawModule, StmtId, StmtNode};

#[derive(Clone, Debug)]
pub struct Program {
    pub root_module_name: IdentId,
    pub root_file_id: FileId,
    pub modules: HashMap<FileId, RawModule>,
    pub dependency_graph: Graph<FileId>,
}

impl Program {
    pub fn new(
        root_module_name: IdentId,
        root_file_id: FileId,
        modules: HashMap<FileId, RawModule>,
        dependency_graph: Graph<FileId>,
    ) -> Self {
        Self {
            root_module_name,
            root_file_id,
            modules,
            dependency_graph,
        }
    }

    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(&self, visitor: &mut V) {
        visitor.visit_program(self)
    }
}
