#![feature(try_trait_v2)]
pub mod error;
pub mod resolver;
pub mod state;
pub mod visitor;

use error::{Error, Result};
use qed_ast::*;
use qed_builder::{Context, ContextFelt, ContextInput};
use std::path::Path;

use crate::symbol_table::SymbolTable;

#[derive(Debug)]
pub struct Interpreter<F, C> {
    inputs: Vec<u64>,
    symbols: SymbolTable<IdentId, ValueNode<F>>,
    ctx: C,
    arena: Arena<F>,
}

impl<F: ContextFelt, C: Context<F>> ContextInput for Interpreter<F, C> {
    fn get_input(&self, index: u64) -> u64 {
        self.inputs[index as usize]
    }
}

impl<F: ContextFelt, C: Context<F>> Interpreter<F, C> {
    pub fn new(ctx: C) -> Self {
        Self {
            inputs: vec![],
            symbols: SymbolTable::new(),
            ctx,
            arena: Arena::new(),
        }
    }

    pub fn run<P: AsRef<Path>>(&mut self, path: &P) -> Result<Vec<StmtNode>> {
        let contents = std::fs::read_to_string(path.as_ref())?;
        let mut stmts = qed_parser::parse::<F, C>(&contents, &mut self.arena, &mut self.ctx)
            .map_err(|err| Error::ParseError(err.to_string()))?;
        for stmt in &mut stmts {
            stmt.accept_visitor(self);
        }
        Ok(stmts)
    }
}

#[cfg(test)]
mod test {
    use qed_builder::{ContextEval, ExecContext, SymFeltEvalCache, SymFeltRef, SymFeltStore};

    use super::*;

    #[test]
    fn test_interpreter() {
        insta::glob!("../../tests", "002.qed", |path| {
            let mut interpreter = Interpreter::<SymFeltRef, _>::new(ExecContext::new());
            let mut cache = SymFeltEvalCache::new();
            let store = SymFeltStore::new();
            println!("{:#?}", interpreter.run(&path).unwrap());
            eprintln!("DEBUGPRINT[1]: lib.rs:59: store={:#?}", store);
            // store.resolve_felt_ref_cached(SymFeltRef(0), &interpreter, &mut cache);
        });
    }
}
