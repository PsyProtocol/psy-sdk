use std::marker::PhantomData;

use qed_ast::*;

use MutAstVisitor;

#[derive(Clone, Debug)]
pub struct StorageProcessor<F: Clone, C> {
    _marker: PhantomData<(F, C)>,
}

impl<F: Clone, C> StorageProcessor<F, C> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreprocessorContext {}

impl PreprocessorContext {
    pub fn new() -> Self {
        Self {}
    }
}

impl<F: Clone, C> MutAstVisitor<F, C> for StorageProcessor<F, C> {
    type Context = PreprocessorContext;

    type Error = ();

    fn visit_use(&mut self, u: &mut UsePath, ctx: &mut Self::Context) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_path(
        &mut self,
        node: &mut PathNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_index_access(
        &mut self,
        node: &mut IndexAccessNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_member_access(
        &mut self,
        node: &mut MemberAccessNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_value(
        &mut self,
        node: &mut ValueNode<F>,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_binary(
        &mut self,
        node: &mut BinaryNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_unary(
        &mut self,
        node: &mut UnaryNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_call(
        &mut self,
        node: &mut CallNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_cast(
        &mut self,
        node: &mut CastNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_if(&mut self, node: &mut IfNode, ctx: &mut Self::Context) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_while(
        &mut self,
        node: &mut WhileNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_block(
        &mut self,
        node: &mut BlockNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_assignment(
        &mut self,
        node: &mut AssignmentNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_variable(
        &mut self,
        node: &mut VariableNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_return(
        &mut self,
        expr: &mut ReturnNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_impl(
        &mut self,
        node: &mut ImplNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_trait(
        &mut self,
        node: &mut TraitNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_function(
        &mut self,
        node: &mut FunctionNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_struct(
        &mut self,
        node: &mut StructNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_enum(
        &mut self,
        node: &mut EnumNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
