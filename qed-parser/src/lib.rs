pub mod error;

use std::{collections::HashMap, path::PathBuf};

use lalrpop_util::lalrpop_mod;

use error::{Error, Result};
use qed_ast::*;
use qed_builder::{Context, ContextFelt};
use qed_common::*;
use qed_lexer::{Error as LexicalError, *};

use qed_ast::Program;

pub type Loc = usize;
pub type ParseError<'input> = lalrpop_util::ParseError<Loc, Token<'input>, LexicalError>;

lalrpop_mod!(pub qed);

#[derive(Debug)]
pub struct Parser<F: Clone, C> {
    pub file_resolver: FileResolver,
    pub exprs: Arena<ExprId, ExprNode<F>>,
    pub stmts: Arena<StmtId, StmtNode<F>>,
    pub interner: Interner,
    _marker: std::marker::PhantomData<C>,
}

impl<F: ContextFelt, C: Context<F>> Parser<F, C> {
    pub fn new() -> Self {
        Self {
            file_resolver: FileResolver::new(),
            exprs: Arena::new(),
            stmts: Arena::new(),
            interner: Interner::new(),
            _marker: std::marker::PhantomData,
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
    pub fn parse<'input>(&'input mut self, ctx: &mut C, entry: PathBuf) -> Result<'input, Program> {
        let mut parsed_modules = HashMap::new();
        let mut dependency_graph: Graph<FileId> = Graph::new();
        let mut module_stack = vec![(entry.clone(), None)];

        while let Some((current_path, parent_file_id)) = module_stack.pop() {
            let file_id = self.file_resolver.resolve_file(current_path.clone())?;
            let module_name = self
                .interner
                .intern_ident(current_path.file_stem().and_then(|s| s.to_str()).unwrap());
            eprintln!("DEBUGPRINT[1]: lib.rs:55: current_path={:#?}", current_path);

            if let Some(parent) = parent_file_id {
                dependency_graph.add_edge(parent, file_id);
            }

            if parsed_modules.contains_key(&file_id) {
                continue;
            }

            let file_content = self
                .file_resolver
                .resolve_content(&file_id)
                .ok_or(Error::FileUnresolved)?;

            let lexer = Lexer::new(file_content);
            let module = qed::ModuleParser::new().parse(
                file_content,
                file_id,
                module_name,
                parent_file_id,
                &mut self.exprs,
                &mut self.stmts,
                &mut self.interner,
                module_name == IdentId::STD || module_name == IdentId::PRELUDE,
                module_name == IdentId::PRELUDE,
                ctx,
                lexer,
            )?;
            eprintln!("DEBUGPRINT[2]: lib.rs:69: module={:#?}", module);

            for dep_module in &module.modules {
                let dep_path = self.resolve_module_path(dep_module, &current_path).unwrap();
                module_stack.push((dep_path, Some(file_id)));
            }

            parsed_modules.insert(file_id, module);
        }

        if dependency_graph.has_cycle() {
            return Err(Error::CycleDependency);
        }

        Ok(Program::new(
            self.interner
                .intern_ident(entry.file_stem().and_then(|s| s.to_str()).unwrap()),
            self.file_resolver.resolve_id(&entry).cloned().unwrap(),
            parsed_modules,
            dependency_graph,
        ))
    }

    fn resolve_module_path(
        &self,
        module_name: &IdentId,
        current_path: &PathBuf,
    ) -> Option<PathBuf> {
        if module_name == &IdentId::STD {
            let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
            let mut std_path = PathBuf::from(cargo_dir);
            return Some(std_path.join("../qed-std/std.qed"));
        }

        let mut path = current_path.parent()?.to_path_buf();
        let ext = current_path.extension()?.to_str()?;
        path.push(format!("{}.{}", self.interner[module_name.clone()], ext));
        Some(path)
    }
}
