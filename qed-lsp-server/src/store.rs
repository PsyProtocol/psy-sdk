use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use dashmap::DashMap;
use qedlang_core::dpn::ops::context_trait::{ContextFelt, DPNContext};
use qed_ast::{AstVisitor, Program, Span};

pub struct ProgramStore<F: Clone + From<u32>> {
    pub map: DashMap<PathBuf, Arc<Program<F>>>,
}

impl<F: Clone + From<u32>> ProgramStore<F> {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    pub fn insert(&self, path: PathBuf, program: Program<F>) {
        self.map.insert(path, Arc::new(program));
    }

    pub fn get(&self, path: &PathBuf) -> Option<Arc<Program<F>>> {
        self.map.get(path).map(|entry| Arc::clone(entry.value()))
    }

    pub fn remove(&self, path: &PathBuf) {
        self.map.remove(path);
    }

    pub fn contains(&self, path: &PathBuf) -> bool {
        self.map.contains_key(path)
    }
}


use tower_lsp::lsp_types::{Position, Range, Url};
use qed_parser::Parser;
use qed_sema::{CheckedExprNode, CheckedProgram, Evaluator, TypeChecker, TypeCheckerVisitorContext};
use crate::core::SymbolInfo;

pub struct AnalysisCache<F: Clone + From<u32> + ContextFelt, C: Clone + DPNContext<F> + Evaluator<F,C> +'static > {
    pub checked_programs: DashMap<Url, CheckedProgram<F>>,
    pub type_contexts: DashMap<Url, TypeCheckerVisitorContext<F, C>>,
    pub symbol_ranges: DashMap<Url, Vec<SymbolInfo>>,
}

impl<F: Clone + From<u32> + ContextFelt, C: Clone + DPNContext<F> + Evaluator<F,C> +'static > AnalysisCache<F, C> {
    pub fn new() -> Self {
        Self {
            checked_programs: DashMap::new(),
            type_contexts: DashMap::new(),
            symbol_ranges: DashMap::new(),
        }
    }

    pub fn reload(&self, ctx: &mut C, uri: Url, text: &str) -> anyhow::Result<()> {
        let path = uri
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("Invalid URI path"))?;

        // Step 1: If the compilation result of the URI already exists in the cache, skip
        if self.checked_programs.contains_key(&uri) && self.type_contexts.contains_key(&uri) {
            return Ok(());
        }

        // Step 2: Parse and type check
        let mut program = Program::new();
        let mut parser = Parser::new(&mut program);

        parser
            //todo: replace with text from editor
            .parse(ctx, path.clone())
            .map_err(|err| anyhow::anyhow!("Parse error: {}", err))?;

        let mut typechecker =
            TypeChecker::<F, C>::new(CheckedProgram::new(), Box::new(ctx.clone()));
        let mut typechecker_ctx = TypeCheckerVisitorContext::new(program);

        typechecker.visit_program(&mut typechecker_ctx)?;
        let checked_program = typechecker.program;

        // Step 3: Traverse the expression nodes of type Path
        let mut symbols = vec![];
        for expr_node in &checked_program.exprs {
            if let CheckedExprNode::Path(path_node) = expr_node {
                let range = span_to_range(&path_node.span, text);
                //todo! replace with real type name
                let type_name = format!("{:?}", path_node.type_id);
                let definition = format!("{:?}", path_node.scope_id);
                symbols.push(SymbolInfo {
                    range,
                    path: path_node.clone(),
                    type_name,
                    definition,
                    //todo! replace with real documentation
                    documentation: "".to_string(),
                });
            }
        }

        // Step 4: Update the cache
        self.checked_programs.insert(uri.clone(), checked_program);
        self.symbol_ranges.insert(uri.clone(), symbols);
        self.type_contexts.insert(uri.clone(), typechecker_ctx);

        Ok(())
    }
    /// Given URI and position, find the matching SymbolInfo
    pub fn get_symbol_info(&self, uri: &Url, position: Position) -> Option<SymbolInfo> {
        self.symbol_ranges.get(uri).and_then(|symbols| {
            symbols
                .iter()
                .find(|symbol| position_in_range(position, symbol.range))
                .cloned()
        })
    }
}

/// Convert Span to LSP Range (requires original file content)
fn span_to_range(span: &Span, source: &str) -> Range {
    fn offset_to_position(offset: usize, text: &str) -> Position {
        let mut line = 0;
        let mut col = 0;
        let mut current = 0;

        for l in text.lines() {
            let line_len = l.len() + 1; // +1 for newline
            if current + line_len > offset {
                col = offset - current;
                break;
            }
            current += line_len;
            line += 1;
        }

        Position {
            line: line as u32,
            character: col as u32,
        }
    }

    Range {
        start: offset_to_position(span.start, source),
        end: offset_to_position(span.end, source),
    }
}

/// Determine if Position is within Range
fn position_in_range(pos: Position, range: tower_lsp::lsp_types::Range) -> bool {
    let start = range.start;
    let end = range.end;

    (pos.line > start.line || (pos.line == start.line && pos.character >= start.character)) &&
        (pos.line < end.line || (pos.line == end.line && pos.character <= end.character))
}