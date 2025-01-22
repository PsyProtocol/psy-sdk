use std::marker::PhantomData;

use indexmap::IndexMap;
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

    fn generate_storage_impl<F: Clone + From<u32>, C, V: VisitorContext<F, C>>(
        &self,
        struct_node: &StructNode,
        ctx: &mut V,
    ) -> ImplNode {
        let mut methods = Vec::new();

        methods.push(self.generate_storage_size_method(struct_node, ctx));

        methods.push(self.generate_storage_read_method(struct_node, ctx));

        methods.push(self.generate_storage_write_method(struct_node, ctx));

        ImplNode {
            generic_parameters: vec![],
            trait_name: Some(ctx.intern("Storage")),
            ty: struct_node.name,
            body: methods,
        }
    }

    fn generate_accessor_impl<F: Clone + From<u32>, C, V: VisitorContext<F, C>>(
        &self,
        struct_node: &StructNode,
        ctx: &mut V,
    ) -> ImplNode {
        let mut methods = Vec::new();
        let mut offset = ctx.alloc_expression(ExprNode::Value(ValueNode::Felt(F::from(0))));

        for (field_name, field_type) in &struct_node.fields {
            methods.push(self.generate_getter(field_name, field_type, offset, ctx));

            methods.push(self.generate_setter(field_name, field_type, offset, ctx));

            let node = ExprNode::Binary(BinaryNode {
                lhs: offset,
                operator: BinaryOperator::Add,
                rhs: self.generate_field_size(field_type, ctx),
            });
            offset = ctx.alloc_expression(node);
        }

        ImplNode {
            generic_parameters: vec![],
            trait_name: None,
            ty: struct_node.name,
            body: methods,
        }
    }

    fn generate_storage_size_method<F: Clone + From<u32>, C, V: VisitorContext<F, C>>(
        &self,
        struct_node: &StructNode,
        ctx: &mut V,
    ) -> DefId {
        let mut sum = self.generate_field_size(&struct_node.fields[0].1, ctx);
        for (field_name, field_type) in struct_node.fields.iter().skip(1) {
            let node = BinaryNode {
                lhs: sum,
                operator: BinaryOperator::Add,
                rhs: self.generate_field_size(field_type, ctx),
            };
            sum = ctx.alloc_expression(ExprNode::Binary(node));
        }

        let return_stmt = ctx.alloc_statement(StmtNode::Return(ReturnNode(Some(sum))));
        let block = ctx.alloc_statement(StmtNode::Block(BlockNode {
            stmts: vec![return_stmt],
        }));

        let f = FunctionNode {
            name: ctx.intern("size"),
            parameters: vec![],
            generic_parameters: vec![],
            body: Some(block),
            return_type: Some(UncheckedType::Basic(IdentId::TYPE_FELT)),
            is_extern: false,
        };

        ctx.alloc_definition(DefinitionNode::Function(f))
    }

    fn generate_storage_read_method<F: Clone + From<u32>, C, V: VisitorContext<F, C>>(
        &self,
        struct_node: &StructNode,
        ctx: &mut V,
    ) -> DefId {
        let offset_ident = ctx.intern("offset");
        let mut offset = ctx.alloc_expression(ExprNode::Path(PathNode {
            path_type: PathType::Basic,
            root: None,
            segments: vec![],
            target: offset_ident,
        }));
        let mut field_reads = IndexMap::new();

        for (field_name, field_type) in &struct_node.fields {
            let (key, value) = self.generate_field_read(field_name, field_type, offset, ctx);
            field_reads.insert(field_name.clone(), value);
            let node = BinaryNode {
                lhs: offset,
                operator: BinaryOperator::Add,
                rhs: self.generate_field_size(field_type, ctx),
            };
            offset = ctx.alloc_expression(ExprNode::Binary(node));
        }

        let value_node = ctx.alloc_expression(ExprNode::Value(ValueNode::Struct(
            struct_node.name,
            vec![],
            field_reads,
        )));
        let return_stmt = ctx.alloc_statement(StmtNode::Return(ReturnNode(Some(value_node))));
        let block = ctx.alloc_statement(StmtNode::Block(BlockNode {
            stmts: vec![return_stmt],
        }));
        let f = FunctionNode {
            name: ctx.intern("read"),
            parameters: vec![(
                offset_ident,
                false,
                UncheckedType::Basic(IdentId::TYPE_FELT),
            )],
            generic_parameters: vec![],
            body: Some(block),
            return_type: Some(UncheckedType::Basic(IdentId::TYPE_SELF)),
            is_extern: false,
        };

        ctx.alloc_definition(DefinitionNode::Function(f))
    }

    fn generate_storage_write_method<F: Clone + From<u32>, C, V: VisitorContext<F, C>>(
        &self,
        struct_node: &StructNode,
        ctx: &mut V,
    ) -> DefId {
        let offset_ident = ctx.intern("offset");
        let value_ident = ctx.intern("value");
        let mut offset = ctx.alloc_expression(ExprNode::Path(PathNode {
            path_type: PathType::Basic,
            root: None,
            segments: vec![],
            target: offset_ident,
        }));
        let mut field_writes = Vec::new();

        for (field_name, field_type) in &struct_node.fields {
            let stmt_id = self.generate_field_write(field_name, field_type, offset, ctx);
            field_writes.push(stmt_id);
            let node = BinaryNode {
                lhs: offset,
                operator: BinaryOperator::Add,
                rhs: self.generate_field_size(field_type, ctx),
            };
            offset = ctx.alloc_expression(ExprNode::Binary(node));
        }
        let block = ctx.alloc_statement(StmtNode::Block(BlockNode {
            stmts: field_writes,
        }));

        let f = FunctionNode {
            name: ctx.intern("write"),
            parameters: vec![
                (
                    offset_ident,
                    false,
                    UncheckedType::Basic(IdentId::TYPE_FELT),
                ),
                (value_ident, false, UncheckedType::Basic(IdentId::TYPE_SELF)),
            ],
            generic_parameters: vec![],
            body: Some(block),
            return_type: None,
            is_extern: false,
        };

        ctx.alloc_definition(DefinitionNode::Function(f))
    }

    fn generate_getter<F: Clone + From<u32>, C, V: VisitorContext<F, C>>(
        &self,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> DefId {
        todo!("Generate getter method AST node")
    }

    fn generate_setter<F: Clone + From<u32>, C, V: VisitorContext<F, C>>(
        &self,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> DefId {
        todo!("Generate setter method AST node")
    }

    fn generate_field_size<F: Clone + From<u32>, C, V: VisitorContext<F, C>>(
        &self,
        field_type: &UncheckedType,
        ctx: &mut V,
    ) -> ExprId {
        let size_ident = ctx.intern("size");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            path_type: PathType::Nested,
            root: None,
            segments: vec![field_type.as_basic().unwrap().clone()],
            target: size_ident,
        }));
        let node = CallNode {
            variable,
            receiver: None,
            generic_parameters: Vec::new(),
            args: Vec::new(),
        };
        ctx.alloc_expression(ExprNode::Call(node))
    }

    fn generate_field_read<F: Clone + From<u32>, C, V: VisitorContext<F, C>>(
        &self,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> (IdentId, ExprId) {
        let read_ident = ctx.intern("read");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            path_type: PathType::Nested,
            root: None,
            segments: vec![field_type.as_basic().unwrap().clone()],
            target: read_ident,
        }));
        let node = CallNode {
            variable,
            receiver: None,
            generic_parameters: Vec::new(),
            args: vec![offset],
        };
        (
            field_name.clone(),
            ctx.alloc_expression(ExprNode::Call(node)),
        )
    }

    fn generate_field_write<F: Clone + From<u32>, C, V: VisitorContext<F, C>>(
        &self,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> StmtId {
        let value_ident = ctx.intern("value");
        let write_ident = ctx.intern("write");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            path_type: PathType::Nested,
            root: None,
            segments: vec![field_type.as_basic().unwrap().clone()],
            target: write_ident,
        }));
        let value = ctx.alloc_expression(ExprNode::Path(PathNode {
            path_type: PathType::Basic,
            root: None,
            segments: vec![],
            target: value_ident,
        }));
        let field = ctx.alloc_expression(ExprNode::MemberAccess(MemberAccessNode {
            value: value,
            field: field_name.clone(),
        }));
        let node = CallNode {
            variable,
            receiver: None,
            generic_parameters: Vec::new(),
            args: vec![offset, field],
        };
        let node_id = ctx.alloc_expression(ExprNode::Call(node));
        ctx.alloc_statement(StmtNode::Expression(node_id))
    }
}

#[derive(Debug)]
pub struct PreprocessorContext<'a, F: Clone + From<u32>, C> {
    path_stack: Vec<NodeId>,
    program: &'a mut Program<F>,
    _marker: std::marker::PhantomData<(&'a (), F, C)>,
}

impl<'a, F: Clone + From<u32>, C> PreprocessorContext<'a, F, C> {
    pub fn new(program: &'a mut Program<F>) -> Self {
        Self {
            path_stack: Vec::new(),
            program,
            _marker: PhantomData,
        }
    }
}

impl<'a, F: Clone + From<u32>, C> VisitorContext<F, C> for PreprocessorContext<'a, F, C> {
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

    fn intern<S: Into<Ident>>(&mut self, s: S) -> IdentId {
        self.program.interner.intern_ident(s)
    }

    fn module(&self, module_id: ModuleId) -> &ModuleNode {
        self.program.modules[module_id].data()
    }

    fn program(&self) -> &Program<F> {
        &self.program
    }

    fn dependency_graph(&self) -> Graph<ModuleId> {
        self.program.dependency_graph.clone()
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

    fn append_definition(&mut self, definition: DefinitionNode) {
        let def_id = self.program.defs.alloc_item(definition);
        assert!(self.parent_node_type() == NodeType::Module);
        let module_id = self.parent_node_id().as_module().unwrap().clone();
        self.program.modules[module_id]
            .data_mut()
            .definitions
            .push(def_id);
    }

    fn alloc_expression(&mut self, expr: ExprNode<F>) -> ExprId {
        self.program.exprs.alloc_item(expr)
    }

    fn alloc_statement(&mut self, stmt: StmtNode<F>) -> StmtId {
        self.program.stmts.alloc_item(stmt)
    }

    fn alloc_definition(&mut self, definition: DefinitionNode) -> DefId {
        self.program.defs.alloc_item(definition)
    }
}

impl<'a, F: Clone + From<u32> + 'static, C> AstVisitor<F, C> for StorageProcessor<'a> {
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
        let storage_trait_id = ctx.intern("Storage");
        let storage_attribute_id = ctx.intern("storage");

        for attr in &s.attrs {
            if attr.is_derive() && attr.properties.iter().any(|p| p == &storage_trait_id) {
                let impl_node = self.generate_storage_impl(&s, ctx);
                ctx.append_definition(DefinitionNode::Impl(impl_node));
            }

            // if attr.name == storage_attribute_id {
            //     let impl_node = self.generate_accessor_impl(&s, ctx);
            //     ctx.append_definition(DefinitionNode::Impl(impl_node));
            // }
        }

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
