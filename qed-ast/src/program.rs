use std::collections::HashMap;

use qed_common::{Arena, FileId, Graph, Tree};

use crate::{
    AstVisitor, ExprId, ExprNode, IdentId, Interner, ModuleId, RawModule, StmtId, StmtNode,
};

#[derive(Clone, Debug)]
pub struct Program {
    pub modules: Tree<ModuleId, RawModule>,
    pub dependency_graph: Graph<ModuleId>,
}

impl Program {
    pub fn new(modules: Tree<ModuleId, RawModule>, dependency_graph: Graph<ModuleId>) -> Self {
        Self {
            modules,
            dependency_graph,
        }
    }

    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
        &self,
        visitor: &mut V,
        ctx: &mut V::Context,
    ) -> Result<(), V::Error> {
        visitor.visit_program(self, ctx)
    }
}
