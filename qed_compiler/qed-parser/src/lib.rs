pub mod error;

use error::UserError;
pub use error::{Error, Result};
use indexmap::IndexMap;
use lalrpop_util::lalrpop_mod;
use qed_ast::*;
use qed_lexer::{GenericTokenTransformer, Lexer, Loc, Token};
use qedlang_core::dpn::ops::context_trait::{ContextFelt, DPNContext};
use std::path::{Path, PathBuf};

use qed_ast::Program;

pub type LalrpopError<'input> = lalrpop_util::ParseError<Loc, Token<'input>, UserError>;

lalrpop_mod!(pub qed);

#[derive(Debug)]
pub struct Parser<'a, F: Clone + From<u32>, C> {
    program: &'a mut Program<F>,
    _marker: std::marker::PhantomData<C>,
}

impl<'a, F: ContextFelt + From<u32>, C: DPNContext<F>> Parser<'a, F, C> {
    pub fn new(program: &'a mut Program<F>) -> Self {
        Self {
            program,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn find_module_by_name(&self, name: impl Into<IdentId>) -> Option<ModuleId> {
        let name = name.into();
        println!("find_module_by_name: {}", self.program.interner[name].0);
        self.program
            .modules
            .iter()
            .find(|m| m.data().name.id == name)
            .map(|m| m.id())
    }

    pub fn add_module_dependency(&mut self, module: Option<ModuleId>, dep_module: ModuleId) {
        // let module_id = module.unwrap_or(ModuleId::root());
        if let Some(module) = module {
            self.program.dependency_graph.add_edge(module, dep_module);
        } else {
            println!(
                "No parent module specified for dependency: {:?}",
                dep_module
            );
        }
    }

    pub fn add_module_child(&mut self, parent: Option<ModuleId>, child: ModuleId) {
        if let Some(parent) = parent {
            self.program.modules.add_child(parent, child);
        } else {
            println!("No parent module specified for child: {:?}", child);
        }
    }

    // std/
    //     prelude
    //
    // modA/
    //     std/
    //         prelude
    // modB/
    //     std/
    //         prelude
    pub fn parse(&mut self, ctx: &mut C, root_module_path: PathBuf) -> Result<()> {
        let mut module_stack: Vec<(bool, PathBuf, Option<ModuleId>, Visibility, bool, Location)> =
            vec![(
                false,
                root_module_path.clone(),
                None,
                Visibility::Public,
                false,
                Location::default(),
            )];
        let mut visited = IndexMap::new();
        let mut inline_modules: IndexMap<PathBuf, ModuleNode> = IndexMap::new();

        while let Some((
            is_inline,
            current_path,
            parent_module_id,
            visibility,
            is_parent_std,
            location,
        )) = module_stack.pop()
        {
            if let Some(&module_id) = visited.get(&current_path) {
                self.add_module_dependency(parent_module_id, module_id);
                continue;
            }

            let mut module: ModuleNode = if !is_inline {
                let module_name = self.resolve_module_name(&current_path);
                if let Some(module_id) = self.find_module_by_name(module_name) {
                    continue;
                }

                let file_id = self
                    .program
                    .file_resolver
                    .resolve_file(current_path.clone())?;
                let file_content = self
                    .program
                    .file_resolver
                    .resolve_content(&file_id)
                    .ok_or(Error::FileUnresolved)?;

                let is_std = is_parent_std || module_name == IdentId::STD;
                let lexer = Lexer::new(file_content);
                let transformer = GenericTokenTransformer::new(lexer);
                let tokens: Vec<_> = transformer.collect::<qed_lexer::Result<Vec<_>>>()?;
                let module = qed::ModuleParser::new()
                    .parse(
                        file_content,
                        file_id,
                        Identifier::new(
                            module_name,
                            Location::new(file_id, location.start, location.end),
                        ),
                        &mut self.program.exprs,
                        &mut self.program.stmts,
                        &mut self.program.defs,
                        &mut self.program.interner,
                        visibility,
                        is_std,
                        ctx,
                        tokens,
                    )
                    .map_err(|e| Error::from_lalrpop_error(e, file_id))?;
                module
            } else {
                inline_modules
                    .get(&current_path)
                    .expect("Inline module not found")
                    .clone()
            };

            module.definitions.sort_by(|a, b| {
                let a = &self.program.defs[*a];
                let b = &self.program.defs[*b];
                b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Less)
            });

            let module_id = self.program.modules.next_idx();

            if !is_inline {
                self.program.file_resolver.register_module_id(module_id.0);
            }

            for (dep_module, visibility, _location) in module.modules.iter().rev() {
                let dep_path = self
                    .resolve_module_path(&dep_module.id, &current_path)
                    .unwrap();
                module_stack.push((
                    false,
                    dep_path,
                    Some(module_id),
                    visibility.clone(),
                    is_parent_std || module.name == IdentId::STD,
                    dep_module.location,
                ));
            }

            for inline_module in module.inline_modules.iter().rev() {
                let dep_path = self
                    .resolve_module_path(&inline_module.name.id, &current_path)
                    .unwrap();
                module_stack.push((
                    true,
                    dep_path.clone(),
                    Some(module_id),
                    inline_module.visibility.clone(),
                    is_parent_std || module.name == IdentId::STD,
                    inline_module.name.location,
                ));
                inline_modules.insert(dep_path, inline_module.clone());
            }

            self.program.modules.add_node(module);
            self.add_module_child(parent_module_id, module_id);
            self.add_module_dependency(parent_module_id, module_id);

            visited.insert(current_path, module_id);
        }

        // Add modules imported by use statements to the dependency
        for module in self.program.modules.clone().iter() {
            let parent_module_id = module.id();
            let use_module_names = module
                .data()
                .definitions
                .iter()
                .filter_map(|def| {
                    let def_node = &self.program.defs[*def];
                    if let DefinitionNode::Use(node) = def_node {
                        Some(node)
                    } else {
                        None
                    }
                })
                .filter_map(|use_node| {
                    let target = match use_node.target {
                        Some(ref target) => Some(target),
                        None => use_node.segments.last(),
                    };
                    target.map(|t| t.id)
                })
                .collect::<Vec<_>>();

            for use_module_name in use_module_names.into_iter() {
                if let Some(use_module_id) = self.find_module_by_name(use_module_name) {
                    self.add_module_dependency(Some(parent_module_id), use_module_id);
                }
            }
        }

        // Remove duplicates children
        self.program.modules.iter_mut().for_each(|module| {
            let children = module.children_mut();
            children.sort_unstable();
            children.dedup();
        });

        println!("loaded module (symbol)");
        let interner = &self.program.interner;
        for module in self.program.modules.iter() {
            println!(
                "module: {}, {:?}",
                interner[module.data().name.id],
                module.id()
            );
            println!("\tvisibility: {:?}", module.data().visibility);
            println!("\tchildren: ");
            for child in module.children() {
                let child_module = &self.program.modules[*child];
                println!("\t\t{}, {:?}", interner[child_module.data().name.id], child);
            }
            println!("\tdependencies: ");
            if let Some(dependencies) = self.program.dependency_graph.get(&module.id()) {
                for dependency in dependencies.iter() {
                    let dependency_module = &self.program.modules[*dependency];
                    println!(
                        "\t\t{}, {:?}",
                        interner[dependency_module.data().name.id],
                        dependency
                    );
                }
            }
        }

        self.program.dependency_graph.check_cycle::<Error>()?;

        Ok(())
    }

    fn resolve_module_name(&mut self, file_path: &Path) -> IdentId {
        let interner = &mut self.program.interner;
        let file_name_without_extension = file_path.file_stem().and_then(|s| s.to_str()).unwrap();
        let module_name = match file_name_without_extension {
            "lib" | "main" => {
                // Get the parent directory name
                file_path
                    .parent()
                    .and_then(|parent| parent.parent())
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                    .unwrap()
            }
            s => s,
        };
        interner.intern_ident(module_name)
    }

    fn resolve_module_path(
        &self,
        module_name: &IdentId,
        current_path: &PathBuf,
    ) -> Option<PathBuf> {
        if module_name == &IdentId::STD {
            return Some(std_path());
        }

        let mut path = current_path.parent()?.to_path_buf();
        let ext = current_path.extension()?.to_str()?;
        path.push(format!(
            "{}.{}",
            self.program.interner[module_name.clone()],
            ext
        ));
        Some(path)
    }
}

pub fn std_path() -> PathBuf {
    if let Ok(cargo_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let std_path = PathBuf::from(cargo_dir);
        return std_path.join("../qed-std/std.qed");
    }

    let std_path = std::env::var("DARGO_STD_PATH").expect("Cannot find DARGO_STD_PATH");
    return PathBuf::from(std_path);
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use qed_ast::Program;
    use qedlang_core::dpn::ops::exec_context::QExecContext;
    use std::path::PathBuf;
    #[test]
    fn test_qed_parser() {
        let mut program = Program::new();
        let mut parser = Parser::new(&mut program);

        let mut ctx = QExecContext::new();

        let entry_file = PathBuf::from("../tests/storage_test.qed");

        let result = parser.parse(&mut ctx, entry_file);

        match result {
            Ok(program) => {
                println!("{:#?}", program);
            }
            Err(e) => {
                panic!("Parsing failed: {:?}", e);
            }
        }
    }
}
