pub mod error;

use error::UserError;
pub use error::{Error, Result};
use indexmap::IndexMap;
use lalrpop_util::lalrpop_mod;
use qed_ast::*;
use qed_common::Graph;
use qed_lexer::{GenericTokenTransformer, Lexer, Loc, Token};
use qedlang_core::dpn::ops::context_trait::{ContextFelt, DPNContext};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use qed_ast::Program;

pub type LalrpopError<'input> = lalrpop_util::ParseError<Loc, Token<'input>, UserError>;

lalrpop_mod!(pub qed);

#[derive(Debug)]
pub struct Parser<'a, 'b, F: Clone + From<u32>, C> {
    program: &'a mut Program<F>,
    ctx: &'b mut C,
}

impl<'a, 'b, F: ContextFelt + From<u32>, C: DPNContext<F>> Parser<'a, 'b, F, C> {
    pub fn new(program: &'a mut Program<F>, ctx: &'b mut C) -> Self {
        Self { program, ctx }
    }

    fn decouple(&mut self) -> (&mut Program<F>, &mut C) {
        (self.program, self.ctx)
    }

    pub fn program(&self) -> &Program<F> {
        &self.program
    }

    pub fn parse(&mut self, root_module_path: PathBuf) -> Result<ModuleId> {
        let (program, ctx) = self.decouple();
        Self::parse_inner(program, ctx, root_module_path)
    }

    pub fn finish(&mut self, dependency_graph: Graph<CrateId>) -> Result<()> {
        let (program, ctx) = self.decouple();
        Self::finish_inner(program, ctx, dependency_graph)
    }

    fn parse_module(
        program: &mut Program<F>,
        ctx: &mut C,
        current_path: &PathBuf,
        location: Location,
        visibility: Visibility,
    ) -> Result<ModuleNode> {
        let module_name = resolve_module_name(program, current_path);
        let file_id = program.file_resolver.resolve_file(current_path.clone())?;
        let file_content = program
            .file_resolver
            .resolve_content(&file_id)
            .ok_or(Error::FileUnresolved)?;

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
                &mut program.exprs,
                &mut program.stmts,
                &mut program.defs,
                &mut program.interner,
                visibility,
                ctx,
                tokens,
            )
            .map_err(|e| Error::from_lalrpop_error(e, file_id))?;
        Ok(module)
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
    fn parse_inner(
        program: &mut Program<F>,
        ctx: &mut C,
        root_module_path: PathBuf,
    ) -> Result<ModuleId> {
        let mut module_stack = vec![(
            false,
            root_module_path.clone(),
            Option::<ModuleId>::None,
            Visibility::Public,
            Location::default(),
        )];
        let mut visited = HashSet::new();
        let mut inline_modules: IndexMap<PathBuf, ModuleNode> = IndexMap::new();

        let mut entry_module_id = None;
        while let Some((is_inline, current_path, parent_module_id, visibility, location)) =
            module_stack.pop()
        {
            if visited.contains(&current_path) {
                return Err(Error::FileParsedMultipleTimes(current_path.clone()).into());
            }
            let module_id = program.modules.next_idx();
            entry_module_id.get_or_insert(module_id);
            let module: ModuleNode = if !is_inline {
                Self::parse_module(program, ctx, &current_path, location, visibility)?
            } else {
                inline_modules
                    .remove(&current_path)
                    .expect("Inline module not found")
            };

            for (dep_module, visibility, _location) in module.modules.iter().rev() {
                let dep_path = resolve_module_path(program, dep_module.id, &current_path).unwrap();
                module_stack.push((
                    false,
                    dep_path,
                    Some(module_id),
                    visibility.clone(),
                    dep_module.location,
                ));
            }

            for inline_module in module.inline_modules.iter().rev() {
                let dep_path =
                    resolve_module_path(program, inline_module.name, &current_path).unwrap();
                module_stack.push((
                    true,
                    dep_path.clone(),
                    Some(module_id),
                    inline_module.visibility.clone(),
                    inline_module.name.location,
                ));
                inline_modules.insert(dep_path, inline_module.clone());
            }

            program.modules.add_node(module);
            program.add_module_child(parent_module_id, module_id);

            visited.insert(current_path);
        }

        program.dependency_graph.check_cycle::<Error>()?;
        let entry_module_id = entry_module_id.ok_or(Error::NoEntryModule(root_module_path))?;
        Ok(entry_module_id)
    }

    fn finish_inner(
        program: &mut Program<F>,
        ctx: &mut C,
        mut dependency_graph: Graph<CrateId>,
    ) -> Result<()> {
        let std_module_id = Self::parse_inner(program, ctx, std_path())?;
        let std_crate_id = std_module_id.into();
        let crate_ids = dependency_graph
            .nodes()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        for crate_id in crate_ids {
            dependency_graph.add_edge(crate_id, std_crate_id);
        }
        program.dependency_graph = dependency_graph;
        program.dependency_graph.check_cycle::<Error>()?;

        let module_ids = program.modules.iter().map(|n| n.id()).collect::<Vec<_>>();
        for module_id in module_ids {
            if !program.is_module_std(module_id) {
                let file_id = program.modules[module_id].data().file_id;
                let def_id = program.defs.alloc_item(DefinitionNode::Use(UseNode {
                    visibility: Visibility::Private,
                    kind: Identifier::new(IdentId::STD, Location::new(file_id, 0, 0)),
                    segments: vec![Identifier::new(
                        IdentId::PRELUDE,
                        Location::new(file_id, 0, 0),
                    )],
                    target: None,
                    comments: vec![],
                    location: Location::new(file_id, 0, 0),
                }));
                let definitions = &mut program.modules[module_id].data_mut().definitions;
                definitions.insert(0, def_id);
                definitions.sort_by(|a, b| {
                    let a = &program.defs[*a];
                    let b = &program.defs[*b];
                    b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Less)
                });
            }
        }

        // Remove duplicates children
        program.modules.iter_mut().for_each(|module| {
            let children = module.children_mut();
            children.sort_unstable();
            children.dedup();
        });

        program.print_module_graph();
        Ok(())
    }
}

pub fn resolve_module_path<F: Clone + From<u32>>(
    program: &mut Program<F>,
    module_name: impl Into<IdentId>,
    current_path: &PathBuf,
) -> Option<PathBuf> {
    let module_name = module_name.into();
    if module_name == IdentId::STD {
        return Some(std_path());
    }
    let mut path = current_path.parent()?.to_path_buf();
    let ext = current_path.extension()?.to_str()?;
    path.push(format!("{}.{}", program.interner[module_name.clone()], ext));
    Some(path)
}

pub fn resolve_module_name<F: Clone + From<u32>>(
    program: &mut Program<F>,
    file_path: &Path,
) -> IdentId {
    let interner = &mut program.interner;
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

fn std_path() -> PathBuf {
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
        let mut ctx = QExecContext::new();
        let mut parser = Parser::new(&mut program, &mut ctx);

        let entry_file = PathBuf::from("../tests/storage_test.qed");
        let result = parser.parse(entry_file);

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
