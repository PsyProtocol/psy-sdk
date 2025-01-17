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
    pub fn parse<'input>(
        &'input mut self,
        ctx: &mut C,
        root_module_path: PathBuf,
    ) -> Result<'input, Program> {
        let mut modules: Tree<ModuleId, ModuleNode> = Tree::new();
        let mut module_stack: Vec<(PathBuf, Option<ModuleId>)> =
            vec![(root_module_path.clone(), None)];
        let mut visited = HashMap::new();

        while let Some((current_path, parent_module_id)) = module_stack.pop() {
            let file_id = self.file_resolver.resolve_file(current_path.clone())?;
            let module_name = Self::resolve_module_name(&mut self.interner, &current_path);

            if let Some(&module_id) = visited.get(&file_id) {
                modules.add_child(parent_module_id, module_id);
            } else {
                let file_content = self
                    .file_resolver
                    .resolve_content(&file_id)
                    .ok_or(Error::FileUnresolved)?;

                let is_self_std = module_name == IdentId::STD;
                let is_self_prelude = module_name == IdentId::PRELUDE;
                let is_std = parent_module_id
                    .map(|id| modules[id].data().name == IdentId::STD)
                    .unwrap_or(false)
                    || is_self_std;

                let lexer = Lexer::new(file_content);
                let module = qed::ModuleParser::new().parse(
                    file_content,
                    file_id,
                    module_name,
                    &mut self.exprs,
                    &mut self.stmts,
                    &mut self.interner,
                    is_std,
                    is_self_std,
                    is_self_prelude,
                    ctx,
                    lexer,
                )?;
                let module_id = modules.next_idx();

                for dep_module in &module.modules {
                    let dep_path = self.resolve_module_path(dep_module, &current_path).unwrap();
                    module_stack.push((dep_path, Some(module_id)));
                }
                modules.add_node(module);
                modules.add_child(parent_module_id, module_id);

                visited.insert(file_id, module_id);
            }
        }

        let dependency_graph = modules.to_graph();

        if dependency_graph.has_cycle() {
            return Err(Error::CycleDependency);
        }

        Ok(Program::new(modules, dependency_graph))
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
        path.push(format!("{}.{}", self.interner[module_name.clone()], ext));
        Some(path)
    }
}
