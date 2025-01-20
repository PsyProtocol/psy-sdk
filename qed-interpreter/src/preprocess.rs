use std::marker::PhantomData;

use qed_ast::*;

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

impl<F: Clone, C> VisitorContext<F, C> for PreprocessorContext {
    type Artifact = ();

    fn node_id(&self) -> NodeId {
        todo!()
    }

    fn parent_node_id(&self) -> NodeId {
        todo!()
    }

    fn node_path(&self) -> &[NodeId] {
        todo!()
    }

    fn push_node_id(&mut self, node_id: NodeId) {
        todo!()
    }

    fn pop_node_id(&mut self) {
        todo!()
    }

    fn node_type(&self) -> NodeType {
        todo!()
    }

    fn ident(&self, id: IdentId) -> &str {
        todo!()
    }

    fn expression(&self, expr_id: ExprId) -> &ExprNode<F> {
        todo!()
    }

    fn statement(&self, stmt_id: StmtId) -> &StmtNode {
        todo!()
    }

    fn definition(&self, def_id: DefId) -> &DefinitionNode {
        todo!()
    }
}

impl<F: Clone, C> AstVisitor<F, C> for StorageProcessor<F, C> {
    type Context = PreprocessorContext;
    type ExprResult = ();
    type StmtResult = ();
    type Error = ();

    fn visit_use(
        &mut self,
        use_path: &UsePath,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn visit_path(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        todo!()
    }

    fn visit_index_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        todo!()
    }

    fn visit_member_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        todo!()
    }

    fn visit_value(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        todo!()
    }

    fn visit_binary(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        todo!()
    }

    fn visit_unary(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        todo!()
    }

    fn visit_call(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        todo!()
    }

    fn visit_cast(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        todo!()
    }

    fn visit_if(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    fn visit_while(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    fn visit_block(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    fn visit_assignment(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    fn visit_variable(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    fn visit_return(
        &mut self,
        expr: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    fn visit_impl(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    fn visit_trait(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    fn visit_function(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    fn visit_struct(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    fn visit_enum(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }
}
