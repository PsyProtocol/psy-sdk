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
        let mut parsed_modules: HashMap<FileId, RawModule> = HashMap::new();
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

            let is_self_std = module_name == IdentId::STD;
            let is_self_prelude = module_name == IdentId::PRELUDE;
            let is_std = parent_file_id
                .and_then(|id| parsed_modules.get(&id))
                .map(|m| m.name == IdentId::STD)
                .unwrap_or(false)
                || is_self_std;

            let lexer = Lexer::new(file_content);
            let module = match qed::ModuleParser::new().parse(
                file_content,
                module_name,
                parent_file_id,
                &mut self.exprs,
                &mut self.stmts,
                &mut self.interner,
                is_std,
                is_self_std,
                is_self_prelude,
                ctx,
                lexer,
            ){
                Ok(module) => module,
                Err(e) => {
                  
                    print_parse_error(file_content, &e);
                    panic!("Parsing failed:");
                }
            };
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
fn format_error_message(message: &str) -> String {
    message
        .replace("\"", "") 
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
            eprintln!("Error: Parsing failed.\nContext:\n{}", extract_context(file_content, 0, 2));
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
    use std::path::PathBuf;
    use qed_builder::ExecContext;
    use super::{Parser}; // 替换为你的 crate 名称和上下文实现模块路径

    #[test]
    fn test_qed_parser() {

        let mut parser = Parser::new();


        let mut ctx = ExecContext::new();


        let entry_file = PathBuf::from("../tests/003.qed");


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