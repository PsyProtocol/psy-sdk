use crate::{arena::IdentId, ValueNode};

pub trait Resolver<F> {
    fn enter_scope(&mut self);

    fn exit_scope(&mut self);

    fn enter_function(&mut self);

    fn exit_function(&mut self);

    fn define_variable(&mut self, name: &IdentId, value: ValueNode<F>);

    fn set_variable(&mut self, name: IdentId, value: ValueNode<F>);

    fn resolve_variable(&mut self, name: &IdentId) -> ValueNode<F>;
}
