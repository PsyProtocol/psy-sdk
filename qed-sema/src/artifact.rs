use std::ops::{Index, IndexMut};

use qed_ast::{DefId, DefinitionNode, ExprId, ExprNode, Ident, IdentId, Program, StmtId, StmtNode};
use qed_parser::Parser;

#[derive(Debug)]
pub struct Artifact<F: Clone, C> {
    pub parser: Parser<F, C>,
    pub program: Program,
}

impl<F: Clone, C> Artifact<F, C> {
    pub fn new(parser: Parser<F, C>, program: Program) -> Self {
        Self { parser, program }
    }
}

macro_rules! impl_index {
    ($index_type:ty, $output_type:ty, $field:ident) => {
        impl<F: Clone, C> Index<$index_type> for Artifact<F, C> {
            type Output = $output_type;
            fn index(&self, index: $index_type) -> &Self::Output {
                &self.parser.$field[index]
            }
        }

        impl<F: Clone, C> IndexMut<$index_type> for Artifact<F, C> {
            fn index_mut(&mut self, index: $index_type) -> &mut Self::Output {
                &mut self.parser.$field[index]
            }
        }
    };
}

impl_index!(ExprId, ExprNode<F>, exprs);
impl_index!(StmtId, StmtNode, stmts);
impl_index!(DefId, DefinitionNode, defs);
impl_index!(IdentId, Ident, interner);
