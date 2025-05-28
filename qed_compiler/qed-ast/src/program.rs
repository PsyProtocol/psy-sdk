use std::ops::{Index, IndexMut};

use qed_common::{Arena, FileResolver, Graph, Tree, TreeNode};

use crate::{
    DefId, DefinitionNode, ExprId, ExprNode, FileLocation, Ident, IdentId, Interner, Location,
    ModuleId, ModuleNode, StmtId, StmtNode,
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

macro_rules! impl_index {
    ($index_type:ty, $output_type:ty, $field:ident) => {
        impl<F: Clone + From<u32>> Index<$index_type> for Program<F> {
            type Output = $output_type;
            fn index(&self, index: $index_type) -> &Self::Output {
                &self.$field[index]
            }
        }

        impl<F: Clone + From<u32>> IndexMut<$index_type> for Program<F> {
            fn index_mut(&mut self, index: $index_type) -> &mut Self::Output {
                &mut self.$field[index]
            }
        }
    };
}

impl_index!(ExprId, ExprNode<F>, exprs);
impl_index!(StmtId, StmtNode, stmts);
impl_index!(DefId, DefinitionNode, defs);
impl_index!(IdentId, Ident, interner);
impl_index!(ModuleId, TreeNode<ModuleId, ModuleNode>, modules);

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

    pub fn convert_location(&self, location: &Location) -> FileLocation {
        let path = self
            .file_resolver
            .resolve_path(&location.file_id)
            .unwrap()
            .display()
            .to_string();
        FileLocation {
            path: path,
            start: location.start,
            end: location.end,
        }
    }

    pub fn module_name(&self, module_id: impl Into<ModuleId>) -> &Ident {
        let module_id = module_id.into();
        let module = self.modules[module_id].data();
        &self.interner[module.name.id]
    }

    pub fn print_module_graph(&self) {
        println!("[Program modules]");
        let interner = &self.interner;
        for module in self.modules.iter() {
            println!(
                "module: {}, {:?}",
                // self.module_name(module_id),
                interner[module.data().name.id],
                module.id()
            );
            println!("  visibility: {:?}", module.data().visibility);
            println!("  children: ");
            for child in module.children() {
                let child_module = &self.modules[*child];
                println!("    {}, {:?}", interner[child_module.data().name.id], child);
            }
            println!("  dependencies: ");
            if let Some(dependencies) = self.dependency_graph.get(&module.id()) {
                for dependency in dependencies.iter() {
                    let dependency_module = &self.modules[*dependency];
                    println!(
                        "    {}, {:?}",
                        interner[dependency_module.data().name.id],
                        dependency
                    );
                }
            }
        }
    }
}
