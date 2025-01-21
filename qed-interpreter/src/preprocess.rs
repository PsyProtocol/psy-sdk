use std::marker::PhantomData;

use qed_ast::*;
use qed_common::Graph;

#[derive(Debug)]
pub struct StorageProcessor<'a> {
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> StorageProcessor<'a> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct PreprocessorContext<'a, F: Clone, C> {
    path_stack: Vec<NodeId>,
    program: &'a mut Program<F>,
    _marker: std::marker::PhantomData<(&'a (), F, C)>,
}

impl<'a, F: Clone, C> PreprocessorContext<'a, F, C> {
    pub fn new(program: &'a mut Program<F>) -> Self {
        Self {
            path_stack: Vec::new(),
            program,
            _marker: PhantomData,
        }
    }
}

impl<'a, F: Clone, C> VisitorContext<F, C> for PreprocessorContext<'a, F, C> {
    fn node_id(&self) -> NodeId {
        self.path_stack.last().unwrap().clone()
    }

    fn parent_node_id(&self) -> NodeId {
        self.path_stack[self.path_stack.len() - 2].clone()
    }

    fn node_path(&self) -> &[NodeId] {
        &self.path_stack
    }

    fn push_node_id(&mut self, node_id: NodeId) {
        self.path_stack.push(node_id);
    }

    fn pop_node_id(&mut self) {
        self.path_stack.pop();
    }

    fn node_type(&self) -> NodeType {
        match self.node_id() {
            NodeId::Expr(expr_id) => self.expression(expr_id).node_type(),
            NodeId::Stmt(stmt_id) => self.statement(stmt_id).node_type(),
            NodeId::Def(def_id) => self.definition(def_id).node_type(),
            NodeId::Module(_) => NodeType::Module,
        }
    }

    fn parent_node_type(&self) -> NodeType {
        match self.parent_node_id() {
            NodeId::Expr(expr_id) => self.expression(expr_id).node_type(),
            NodeId::Stmt(stmt_id) => self.statement(stmt_id).node_type(),
            NodeId::Def(def_id) => self.definition(def_id).node_type(),
            NodeId::Module(_) => NodeType::Module,
        }
    }

    fn ident(&self, id: IdentId) -> &Ident {
        &self.program.interner[id]
    }

    fn module(&self, module_id: ModuleId) -> &ModuleNode {
        self.program.modules[module_id].data()
    }

    fn program(&self) -> &Program<F> {
        &self.program
    }

    fn expression(&self, expr_id: ExprId) -> &ExprNode<F> {
        &self.program.exprs[expr_id]
    }

    fn statement(&self, stmt_id: StmtId) -> &StmtNode<F> {
        &self.program.stmts[stmt_id]
    }

    fn definition(&self, def_id: DefId) -> &DefinitionNode {
        &self.program.defs[def_id]
    }

    fn dependency_graph(&self) -> Graph<ModuleId> {
        self.program.dependency_graph.clone()
    }

    fn append_definition(&mut self, definition: DefinitionNode) {
        let def_id = self.program.defs.alloc_item(definition);
        assert!(self.parent_node_type() == NodeType::Module);
        let module_id = self.parent_node_id().as_module().unwrap().clone();
        self.program.modules[module_id]
            .data_mut()
            .definitions
            .push(def_id);
    }
}

impl<'a, F: Clone + 'static, C> AstVisitor<F, C> for StorageProcessor<'a> {
    type Context = PreprocessorContext<'a, F, C>;
    type ExprResult = ();
    type StmtResult = ();
    type Error = ();

    fn visit_use(
        &mut self,
        use_path: &UsePath,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn visit_path(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_index_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_member_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_value(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_binary(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_unary(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_call(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_cast(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_if(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_while(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_block(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_assignment(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_variable(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_return(
        &mut self,
        expr: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_impl(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_trait(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_function(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_struct(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let s: StructNode = ctx.definition(node).as_struct().unwrap().clone();

        // for attr in s {
        //     if attr.is_derive() {
        //         ctx.append_definition(DefinitionNode::Impl)
        //     }
        // }

        Ok(())
    }

    fn visit_enum(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }
}
