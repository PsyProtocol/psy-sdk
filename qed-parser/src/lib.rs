pub mod error;

use std::path::{Path, PathBuf};

use error::UserError;
use indexmap::IndexMap;
use lalrpop_util::lalrpop_mod;

pub use error::{Error, Result};
use qed_ast::*;
use qed_common::FileId;
use qed_lexer::{Error as LexicalError, *};
use qedlang_core::dpn::ops::context_trait::{ContextFelt, DPNContext};
use qedlang_core::dpn::ops::exec_context::QExecContext;

use qed_ast::Program;

pub type LalrpopError<'input> = lalrpop_util::ParseError<Loc, Token<'input>, UserError>;

lalrpop_mod!(pub qed);

use crate::Token;

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

    // std/
    //     prelude
    //
    // modA/
    //     std/
    //         prelude
    // modB/
    //     std/
    //         prelude
    pub fn parse<'input>(&'input mut self, ctx: &mut C, root_module_path: PathBuf) -> Result<()> {
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
                self.program.modules.add_child(parent_module_id, module_id);
                continue;
            }

            let module: ModuleNode = if !is_inline {
                let file_id = self
                    .program
                    .file_resolver
                    .resolve_file(current_path.clone())?;
                let module_name =
                    Self::resolve_module_name(&mut self.program.interner, &current_path);

                let file_content = self
                    .program
                    .file_resolver
                    .resolve_content(&file_id)
                    .ok_or(Error::FileUnresolved)?;

                let is_self_std = module_name == IdentId::STD;
                let is_std = is_parent_std || is_self_std;

                let module_id = self
                    .program
                    .modules
                    .iter()
                    .find(|module| module.data().name == module_name)
                    .map(|module| module.id());
                if module_id
                    .map(|module_id| self.program.modules.add_child(parent_module_id, module_id))
                    .is_some()
                {
                    continue;
                }

                let lexer = Lexer::new(file_content);
                let transformer = GenericTokenTransformer::new(lexer);
                let tokens: Vec<_> = transformer.collect::<qed_lexer::Result<Vec<_>>>()?;
                let module = match qed::ModuleParser::new().parse(
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
                    is_self_std,
                    ctx,
                    tokens,
                ) {
                    Ok(module) => module,
                    Err(e) => {
                        return Err(match e {
                            lalrpop_util::ParseError::InvalidToken { location } => {
                                Error::InvalidToken {
                                    location: Location::new(file_id, location, location + 1),
                                }
                            }
                            lalrpop_util::ParseError::UnrecognizedEof { location, expected } => {
                                Error::UnrecognizedEof {
                                    location: Location::new(file_id, location, location + 1),
                                    expected,
                                }
                            }
                            lalrpop_util::ParseError::UnrecognizedToken {
                                token: (start, token, end),
                                expected,
                            } => Error::UnrecognizedToken {
                                token: token.to_string(),
                                expected: expected,
                                location: Location::new(file_id, start, end),
                            },
                            lalrpop_util::ParseError::ExtraToken {
                                token: (start, token, end),
                            } => Error::ExtraToken {
                                token: token.to_string(),
                                location: Location::new(file_id, start, end),
                            },
                            lalrpop_util::ParseError::User { error } => match error {
                                UserError::LexicalError(error) => Error::LexicalError(error),
                                UserError::CommonError(error) => Error::CommonError(error),
                                UserError::IoError(error) => Error::IoError(error),
                                UserError::FileUnresolved => Error::FileUnresolved,
                                UserError::InvalidModuleName => Error::InvalidModuleName,
                                UserError::ExternFnNotInStd => Error::ExternFnNotInStd,
                                UserError::FunctionBodyMissing => Error::FunctionBodyMissing,
                                UserError::InvalidSelfParameter => Error::InvalidSelfParameter,
                            },
                        })
                    }
                };
                module
            } else {
                inline_modules.get(&current_path).unwrap().clone()
            };

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
            self.program.modules.add_child(parent_module_id, module_id);

            visited.insert(current_path, module_id);
        }

        self.program.dependency_graph = self.program.modules.to_graph();

        self.program.dependency_graph.check_cycle::<Error>()?;

        Ok(())
    }

    fn resolve_module_name(interner: &mut Interner, file_path: &Path) -> IdentId {
        let file_name_without_extension = file_path.file_stem().and_then(|s| s.to_str()).unwrap();
        let module_name = match file_name_without_extension {
            "lib" | "main" => {
                // Get the parent directory name
                file_path
                    .parent()
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
