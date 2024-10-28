use qed_ast::{IdentId, Resolver, ValueNode};
use qed_builder::{Context, ContextFelt};
use tracing::instrument;

use super::Interpreter;

impl<F: ContextFelt, C: Context<F>> Resolver<F> for Interpreter<F, C> {
    #[instrument(skip_all)]
    fn enter_scope(&mut self) {
        self.symbols.start_scope();
    }

    #[instrument(skip_all)]
    fn exit_scope(&mut self) {
        self.symbols.end_scope();
    }

    #[instrument(skip_all)]
    fn define_variable(&mut self, name: &IdentId, value: ValueNode<F>) {
        self.symbols.define_var(name.clone(), value);
    }

    #[instrument(skip_all)]
    fn set_variable(&mut self, name: IdentId, value: ValueNode<F>) {
        self.symbols.set_var(name.clone(), value);
    }

    #[instrument(skip_all)]
    fn resolve_variable(&mut self, name: &IdentId) -> ValueNode<F> {
        self.symbols.get_var(name).unwrap().clone()
    }

    fn enter_function(&mut self) {
        self.symbols.start_function();
    }

    fn exit_function(&mut self) {
        self.symbols.end_function();
    }
}
