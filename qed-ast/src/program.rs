use std::collections::HashMap;

use qed_common::{Arena, FileId, FileResolver, Graph, Tree};

use crate::{
    AstVisitor, DefId, DefinitionNode, ExprId, ExprNode, IdentId, Interner, ModuleId, ModuleNode,
    StmtId, StmtNode,
};

#[derive(Debug)]
pub struct Program<F: Clone> {
    pub modules: Tree<ModuleId, ModuleNode>,
    pub dependency_graph: Graph<ModuleId>,
    pub file_resolver: FileResolver,
    pub exprs: Arena<ExprId, ExprNode<F>>,
    pub stmts: Arena<StmtId, StmtNode>,
    pub defs: Arena<DefId, DefinitionNode>,
    pub interner: Interner,
}

impl<F: Clone> Program<F> {
    pub fn new() -> Self {
        Self {
            modules: Tree::new(),
            dependency_graph: Graph::new(),
            file_resolver: FileResolver::new(),
            exprs: Arena::new(),
            stmts: Arena::new(),
            defs: Arena::new(),
            interner: Interner::new(),
        }
    }

    // pub fn accept_visitor<C, V: AstVisitor<F, C>>(
    //     &self,
    //     visitor: &mut V,
    //     ctx: &mut V::Context,
    // ) -> Result<(), V::Error> {
    //     visitor.visit_program(self, ctx)
    // }
}
