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

    fn generate_storage_impl<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
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

    fn generate_accessor_impl<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        struct_node: &StructNode,
        ctx: &mut V,
    ) -> ImplNode {
        let mut methods = Vec::new();
        let mut offset = ctx.alloc_expression(ExprNode::Value(ValueNode::Felt(F::from(0))));

        for (field_name, field_type, _) in &struct_node.fields {
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

    fn generate_storage_size_method<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        struct_node: &StructNode,
        ctx: &mut V,
    ) -> DefId {
        let mut sum = self.generate_field_size(&struct_node.fields[0].1, ctx);
        for (field_name, field_type, _) in struct_node.fields.iter().skip(1) {
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
            visibility: Visibility::Public,
        };

        ctx.alloc_definition(DefinitionNode::Function(f))
    }

    fn generate_storage_read_method<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        struct_node: &StructNode,
        ctx: &mut V,
    ) -> DefId {
        let offset_ident = ctx.intern("offset");
        let mut offset = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: None,
            segments: vec![],
            target: offset_ident,
        }));
        let mut field_reads = IndexMap::new();

        for (field_name, field_type, _) in &struct_node.fields {
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
            visibility: Visibility::Public,
        };

        ctx.alloc_definition(DefinitionNode::Function(f))
    }

    fn generate_storage_write_method<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        struct_node: &StructNode,
        ctx: &mut V,
    ) -> DefId {
        let offset_ident = ctx.intern("offset");
        let value_ident = ctx.intern("value");
        let mut offset = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: None,
            segments: vec![],
            target: offset_ident,
        }));
        let mut field_writes = Vec::new();

        for (field_name, field_type, _) in &struct_node.fields {
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
            visibility: Visibility::Public,
        };

        ctx.alloc_definition(DefinitionNode::Function(f))
    }

    fn generate_getter<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> DefId {
        let mut getter_name = format!("get_{}", ctx.ident(*field_name));
        let getter_ident = ctx.intern(getter_name.as_str());

        let read_ident = ctx.intern("read");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: Some(field_type.as_basic().unwrap().clone()),
            segments: vec![],
            target: read_ident,
        }));

        let read_call = CallNode {
            variable,
            receiver: None,
            generic_parameters: Vec::new(),
            args: vec![offset],
        };
        let read_expr = ctx.alloc_expression(ExprNode::Call(read_call));

        let return_stmt = ctx.alloc_statement(StmtNode::Return(ReturnNode(Some(read_expr))));

        let block = ctx.alloc_statement(StmtNode::Block(BlockNode {
            stmts: vec![return_stmt],
        }));

        let function = FunctionNode {
            name: getter_ident,
            parameters: vec![(
                ctx.intern("self"),
                false,
                UncheckedType::Basic(IdentId::TYPE_SELF),
            )],
            generic_parameters: vec![],
            body: Some(block),
            return_type: Some(field_type.clone()),
            is_extern: false,
            visibility: Visibility::Public,
        };

        ctx.alloc_definition(DefinitionNode::Function(function))
    }

    fn generate_setter<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> DefId {
        let mut setter_name = format!("set_{}", ctx.ident(*field_name));
        let setter_ident = ctx.intern(setter_name.as_str());

        let value_ident = ctx.intern("value");

        let write_ident = ctx.intern("write");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: Some(field_type.as_basic().unwrap().clone()),
            segments: vec![],
            target: write_ident,
        }));

        let value = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: None,
            segments: vec![],
            target: value_ident,
        }));

        let write_call = CallNode {
            variable,
            receiver: None,
            generic_parameters: Vec::new(),
            args: vec![offset, value],
        };
        let write_expr = ctx.alloc_expression(ExprNode::Call(write_call));

        let write_stmt = ctx.alloc_statement(StmtNode::Expression(write_expr));
        let block = ctx.alloc_statement(StmtNode::Block(BlockNode {
            stmts: vec![write_stmt],
        }));

        let function = FunctionNode {
            name: setter_ident,
            parameters: vec![
                (
                    ctx.intern("self"),
                    false,
                    UncheckedType::Basic(IdentId::TYPE_SELF),
                ),
                (value_ident, false, field_type.clone()),
            ],
            generic_parameters: vec![],
            body: Some(block),
            return_type: None,
            is_extern: false,
            visibility: Visibility::Public,
        };

        ctx.alloc_definition(DefinitionNode::Function(function))
    }

    fn generate_field_size<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        field_type: &UncheckedType,
        ctx: &mut V,
    ) -> ExprId {
        let size_ident = ctx.intern("size");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: Some(field_type.as_basic().unwrap().clone()),
            segments: vec![],
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

    fn generate_field_read<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> (IdentId, ExprId) {
        let read_ident = ctx.intern("read");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: Some(field_type.as_basic().unwrap().clone()),
            segments: vec![],
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

    fn generate_field_write<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> StmtId {
        let value_ident = ctx.intern("value");
        let write_ident = ctx.intern("write");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: Some(field_type.as_basic().unwrap().clone()),
            segments: vec![],
            target: write_ident,
        }));
        let value = ctx.alloc_expression(ExprNode::Path(PathNode {
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

    fn statement(&self, stmt_id: StmtId) -> &StmtNode {
        &self.program.stmts[stmt_id]
    }

    fn definition(&self, def_id: DefId) -> &DefinitionNode {
        &self.program.defs[def_id]
    }

    fn insert_definition(&mut self, definition: DefinitionNode, pos: InsertPosition) {
        let def_id = self.program.defs.alloc_item(definition);
        assert!(self.parent_node_type() == NodeType::Module);
        let module_id = self.parent_node_id().as_module().unwrap().clone();

        match pos {
            InsertPosition::Front => {
                self.program.modules[module_id]
                    .data_mut()
                    .definitions
                    .insert(0, def_id);
            }
            InsertPosition::End => {
                self.program.modules[module_id]
                    .data_mut()
                    .definitions
                    .push(def_id);
            }
            InsertPosition::Before(before) => {
                let idx = self.program.modules[module_id]
                    .data()
                    .definitions
                    .iter()
                    .position(|d| d == before.as_def().unwrap())
                    .unwrap();
                self.program.modules[module_id]
                    .data_mut()
                    .definitions
                    .insert(idx, def_id);
            }
            InsertPosition::After(after) => {
                let idx = self.program.modules[module_id]
                    .data()
                    .definitions
                    .iter()
                    .position(|d| d == after.as_def().unwrap())
                    .unwrap();
                self.program.modules[module_id]
                    .data_mut()
                    .definitions
                    .insert(idx + 1, def_id);
            }
        };
    }

    fn alloc_expression(&mut self, expr: ExprNode<F>) -> ExprId {
        self.program.exprs.alloc_item(expr)
    }

    fn alloc_statement(&mut self, stmt: StmtNode) -> StmtId {
        self.program.stmts.alloc_item(stmt)
    }

    fn alloc_definition(&mut self, definition: DefinitionNode) -> DefId {
        self.program.defs.alloc_item(definition)
    }

    fn replace_definition(&mut self, def_id: DefId, definition: DefinitionNode) {
        self.program.defs.replace_item(def_id, definition);
    }

    fn replace_statement(&mut self, stmt_id: StmtId, statement: StmtNode) {
        self.program.stmts.replace_item(stmt_id, statement);
    }

    type Expr = ExprNode<F>;

    type Stmt = StmtNode;

    type Definition = DefinitionNode;
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
        let defs: Vec<DefId> = ctx.definition(node).as_impl().unwrap().body.clone();
        for def_id in defs {
            self.visit_definition(def_id, ctx)?;
        }
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
        let is_extern = ctx.definition(node).as_function().unwrap().is_extern;
        let function_name = ctx.definition(node).as_function().unwrap().name;
        if !is_extern || ctx.parent_node_type() != NodeType::ImplDef {
            return Ok(());
        }

        let parent_node_id = ctx.parent_node_id().as_def().unwrap().clone();
        let trait_name = ctx.definition(parent_node_id).as_impl().unwrap().trait_name;
        let ty_name = ctx.definition(parent_node_id).as_impl().unwrap().ty;
        let storage_trait_id = ctx.intern("Storage");
        let offset_ident = ctx.intern("offset");
        let value_ident = ctx.intern("value");
        let read_ident = ctx.intern("read");
        let write_ident = ctx.intern("write");

        if trait_name == Some(storage_trait_id) {
            let offset = ctx.alloc_expression(ExprNode::Path(PathNode {
                root: None,
                segments: vec![],
                target: offset_ident,
            }));
            let mut value = ctx.alloc_expression(ExprNode::Path(PathNode {
                root: None,
                segments: vec![],
                target: value_ident,
            }));
            let mut storage_expr_id =
                ctx.alloc_expression(ExprNode::Storage(StorageReadNode { offset }));
            if ty_name == IdentId::TYPE_BOOL {
                storage_expr_id = ctx.alloc_expression(ExprNode::Cast(CastNode {
                    value: storage_expr_id,
                    target_type: UncheckedType::Basic(IdentId::TYPE_BOOL),
                }));
                value = ctx.alloc_expression(ExprNode::Cast(CastNode {
                    value: value,
                    target_type: UncheckedType::Basic(IdentId::TYPE_FELT),
                }));
            }

            let body = ctx.definition(node).as_function().unwrap().body.unwrap();
            let stmt = if function_name == read_ident {
                StmtNode::Return(ReturnNode(Some(storage_expr_id)))
            } else if function_name == write_ident {
                StmtNode::Storage(StorageWriteNode { offset, value })
            } else {
                return Ok(());
            };

            let block = BlockNode {
                stmts: vec![ctx.alloc_statement(stmt)],
            };

            ctx.replace_statement(body, StmtNode::Block(block));
        }

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
                let pos = ctx.node_id().as_def().unwrap().clone();
                ctx.insert_definition(
                    DefinitionNode::Impl(impl_node),
                    InsertPosition::After(pos.into()),
                );
            }

            if attr.name == storage_attribute_id {
                let impl_node = self.generate_accessor_impl(&s, ctx);
                let pos = ctx.node_id().as_def().unwrap().clone();
                ctx.insert_definition(
                    DefinitionNode::Impl(impl_node),
                    InsertPosition::After(pos.into()),
                );
            }
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

    fn visit_storage_read(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        todo!()
    }

    fn visit_storage_write(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    type Expr = ExprNode<F>;

    type Stmt = StmtNode;

    type Definition = DefinitionNode;
}
