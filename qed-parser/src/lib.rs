pub mod error;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

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
pub struct Parser<'a, F: Clone, C> {
    program: &'a mut Program<F>,
    _marker: std::marker::PhantomData<C>,
}

impl<'a, F: ContextFelt, C: Context<F>> Parser<'a, F, C> {
    pub fn new(program: &'a mut Program<F>) -> Self {
        Self {
            program,
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
    pub fn parse<'input>(
        &'input mut self,
        ctx: &mut C,
        root_module_path: PathBuf,
    ) -> Result<'input, ()> {
        let mut module_stack: Vec<(PathBuf, Option<ModuleId>)> =
            vec![(root_module_path.clone(), None)];
        let mut visited = HashMap::new();

        while let Some((current_path, parent_module_id)) = module_stack.pop() {
            let file_id = self
                .program
                .file_resolver
                .resolve_file(current_path.clone())?;
            let module_name = Self::resolve_module_name(&mut self.program.interner, &current_path);

            if let Some(&module_id) = visited.get(&file_id) {
                self.program.modules.add_child(parent_module_id, module_id);
            } else {
                let file_content = self
                    .program
                    .file_resolver
                    .resolve_content(&file_id)
                    .ok_or(Error::FileUnresolved)?;

                let is_self_std = module_name == IdentId::STD;
                let is_self_prelude = module_name == IdentId::PRELUDE;
                let is_std = parent_module_id
                    .map(|id| self.program.modules[id].data().name == IdentId::STD)
                    .unwrap_or(false)
                    || is_self_std;

                let lexer = Lexer::new(file_content);
                let module = qed::ModuleParser::new().parse(
                    file_content,
                    file_id,
                    module_name,
                    &mut self.program.exprs,
                    &mut self.program.stmts,
                    &mut self.program.defs,
                    &mut self.program.interner,
                    is_std,
                    is_self_std,
                    is_self_prelude,
                    ctx,
                    lexer,
                )?;
                let module_id = self.program.modules.next_idx();

                for dep_module in &module.modules {
                    let dep_path = self.resolve_module_path(dep_module, &current_path).unwrap();
                    module_stack.push((dep_path, Some(module_id)));
                }
                self.program.modules.add_node(module);
                self.program.modules.add_child(parent_module_id, module_id);

                visited.insert(file_id, module_id);
            }
        }

        self.program.dependency_graph = self.program.modules.to_graph();

        if self.program.dependency_graph.has_cycle() {
            return Err(Error::CycleDependency);
        }

        Ok(())
    }

    fn resolve_module_name(interner: &mut Interner, file_path: &Path) -> IdentId {
        interner.intern_ident(file_path.file_stem().and_then(|s| s.to_str()).unwrap())
    }

    fn resolve_module_path(
        &self,
        module_name: &IdentId,
        current_path: &PathBuf,
    ) -> Option<PathBuf> {
        if module_name == &IdentId::STD {
            let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or("./qed-cli".to_string());
            let std_path = PathBuf::from(cargo_dir);
            return Some(std_path.join("../qed-std/std.qed"));
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
