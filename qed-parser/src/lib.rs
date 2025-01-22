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
pub struct Parser<'a, F: Clone + From<u32>, C> {
    program: &'a mut Program<F>,
    _marker: std::marker::PhantomData<C>,
}

impl<'a, F: ContextFelt + From<u32>, C: Context<F>> Parser<'a, F, C> {
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
                let module = match qed::ModuleParser::new().parse(
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
                ) {
                    Ok(module) => module,
                    Err(e) => {
                        print_parse_error(file_content, &e);
                        panic!("Parsing failed:");
                    }
                };
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
fn format_error_message(message: &str) -> String {
    message.replace("\"", "")
}
fn print_parse_error<'input>(
    file_content: &'input str,
    error: &lalrpop_util::ParseError<Loc, Token<'input>, LexicalError>,
) {
    match error {
        lalrpop_util::ParseError::InvalidToken { location } => {
            eprintln!(
                "Error: Invalid token at position {}.\nContext:\n{}",
                location,
                extract_context(file_content, *location, 2)
            );
        }
        lalrpop_util::ParseError::UnrecognizedToken { token, expected } => {
            let formatted_expected = expected
                .iter()
                .map(|t| format_error_message(&t))
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!(
                "Error: Unrecognized token '{:?}' at position {}. Expected one of: {:?}\nContext:\n{}",
                token.1,
                token.0,
                formatted_expected,
                extract_context(file_content, token.0, 2)
            );
        }
        lalrpop_util::ParseError::ExtraToken { token } => {
            eprintln!(
                "Error: Extra token '{:?}' found at position {}.\nContext:\n{}",
                token.1,
                token.0,
                extract_context(file_content, token.0, 2)
            );
        }
        lalrpop_util::ParseError::User { error } => {
            eprintln!(
                "Error: Lexical error {:?}.\nContext:\n{}",
                error,
                extract_context(file_content, 0, 2)
            );
        }
        _ => {
            eprintln!(
                "Error: Parsing failed.\nContext:\n{}",
                extract_context(file_content, 0, 2)
            );
        }
    }
}
fn extract_context(file_content: &str, position: usize, context_lines: usize) -> String {
    let lines: Vec<_> = file_content.lines().collect();
    let error_line = file_content[..position].lines().count();

    let start_line: usize = error_line.saturating_sub(context_lines);
    let end_line = (error_line + context_lines).min(lines.len());

    lines[start_line..end_line]
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_number = start_line + i + 1;
            if line_number == error_line {
                format!("{:>4}: {} <-- ERROR HERE", line_number, line)
            } else {
                format!("{:>4}: {}", line_number, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::Parser;
    use qed_ast::Program;
    use qed_builder::ExecContext;
    use std::path::PathBuf;
    #[test]
    fn test_qed_parser() {
        let mut program = Program::new();
        let mut parser = Parser::new(&mut program);

        let mut ctx = ExecContext::new();

        let entry_file = PathBuf::from("../tests/002.qed");

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
