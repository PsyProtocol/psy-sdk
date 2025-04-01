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

    pub fn print_module_name(&self) {
        for module in self.modules.iter() {
            println!(
                "module name: {:?}, module id: {:?}",
                self.interner[module.data().name],
                module.id()
            );
        }
    }

    pub fn print_module_graph(&self) {
        println!("After parse: modules in program");
        for module in self.modules.iter() {
            println!(
                "module name: {:?}, module id: {:?}",
                self.interner[module.data().name],
                module.id()
            );
            for def_id in module.data().definitions.iter() {
                let def_node = &self.defs[*def_id];
                if let DefinitionNode::Use(node) = def_node {
                    let ident_id = node.kind.id;
                    let mut module_id = 0.into();
                    for ii in self.modules.iter() {
                        if ident_id == ii.data().name {
                            module_id = ii.id();
                            break;
                        }
                    }
                    println!(
                        "USE: {:?}, module_id: {:?}",
                        self.interner[ident_id], module_id,
                    );
                }
            }
            println!("module dependencies: {:?}", module.children());
        }
        println!("module graph");
        for (i, j) in self.dependency_graph.iter() {
            println!("{:?} -> {:?}", i, j);
        }
        println!("----------------------");
    }
}
