use std::collections::HashMap;

use qed_common::{Arena, FileId, FileResolver, Graph, Tree};

use crate::{
    AstVisitor, DefId, DefinitionNode, ExprId, ExprNode, IdentId, Interner, ModuleId, ModuleNode,
    StmtId, StmtNode,
};

#[derive(Debug)]
pub struct Program<F: Clone + From<u32>> {
    pub modules: Tree<ModuleId, ModuleNode>,
    pub dependency_graph: Graph<ModuleId>,
    pub file_resolver: FileResolver,
    pub exprs: Arena<ExprId, ExprNode<F>>,
    pub stmts: Arena<StmtId, StmtNode>,
    pub defs: Arena<DefId, DefinitionNode>,
    pub interner: Interner,
}

impl<F: Clone + From<u32>> Program<F> {
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
}
