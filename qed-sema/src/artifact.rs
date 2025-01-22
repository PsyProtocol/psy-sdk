use std::ops::{Index, IndexMut};

use qed_ast::{DefId, DefinitionNode, ExprId, ExprNode, Ident, IdentId, Program, StmtId, StmtNode};
use qed_parser::Parser;

#[derive(Debug)]
pub struct Artifact<F: Clone + From<u32>, C> {
    pub program: Program<F>,
    _marker: std::marker::PhantomData<C>,
}

impl<F: Clone + From<u32>, C> Artifact<F, C> {
    pub fn new(program: Program<F>) -> Self {
        Self {
            program,
            _marker: std::marker::PhantomData,
        }
    }
}

macro_rules! impl_index {
    ($index_type:ty, $output_type:ty, $field:ident) => {
        impl<F: Clone + From<u32>, C> Index<$index_type> for Artifact<F, C> {
            type Output = $output_type;
            fn index(&self, index: $index_type) -> &Self::Output {
                &self.program.$field[index]
            }
        }

        impl<F: Clone + From<u32>, C> IndexMut<$index_type> for Artifact<F, C> {
            fn index_mut(&mut self, index: $index_type) -> &mut Self::Output {
                &mut self.program.$field[index]
            }
        }
    };
}

impl_index!(ExprId, ExprNode<F>, exprs);
impl_index!(StmtId, StmtNode<F>, stmts);
impl_index!(DefId, DefinitionNode, defs);
impl_index!(IdentId, Ident, interner);
