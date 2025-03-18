use std::marker::PhantomData;

use indexmap::IndexMap;
use qed_ast::*;

#[derive(Debug)]
pub struct StorageProcessor<'a> {
    _marker: PhantomData<&'a ()>,
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
        attr: &AttrNode,
        ctx: &mut V,
    ) -> ImplTraitNode {
        let mut methods = Vec::new();

        methods.push(self.generate_storage_size_method(struct_node, attr, ctx));

        methods.push(self.generate_storage_read_method(struct_node, attr, ctx));

        methods.push(self.generate_storage_write_method(struct_node, attr, ctx));

        ImplTraitNode {
            generic_parameters: vec![],
            trait_ty: UncheckedType::Basic(ctx.intern("Storage"), attr.span),
            ty: UncheckedType::Basic(struct_node.name, attr.span),
            body: methods,
            span: attr.span,
        }
    }

    fn generate_accessor_impl<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        struct_node: &StructNode,
        attr: &AttrNode,
        ctx: &mut V,
    ) -> ImplNode {
        let mut methods = Vec::new();
        let mut offset =
            ctx.alloc_expression(ExprNode::Value(ValueNode::Felt(F::from(0), attr.span)));

        for (field_name, field) in &struct_node.fields {
            methods.push(self.generate_getter(attr, field_name, &field.ty, offset, ctx));

            methods.push(self.generate_setter(attr, field_name, &field.ty, offset, ctx));

            let node = ExprNode::Binary(BinaryNode {
                lhs: offset,
                operator: BinaryOperator::Add,
                rhs: self.generate_field_size(attr, &field.ty, ctx),
                span: attr.span,
            });
            offset = ctx.alloc_expression(node);
        }

        ImplNode {
            generic_parameters: vec![],
            ty: UncheckedType::Basic(struct_node.name, attr.span),
            body: methods,
            span: attr.span,
        }
    }

    fn generate_storage_size_method<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        struct_node: &StructNode,
        attr: &AttrNode,
        ctx: &mut V,
    ) -> DefId {
        let mut sum =
            self.generate_field_size(attr, &struct_node.fields.iter().next().unwrap().1.ty, ctx);
        for (_, field) in struct_node.fields.iter().skip(1) {
            let node = BinaryNode {
                lhs: sum,
                operator: BinaryOperator::Add,
                rhs: self.generate_field_size(attr, &field.ty, ctx),
                span: attr.span,
            };
            sum = ctx.alloc_expression(ExprNode::Binary(node));
        }
        let block = ctx.alloc_expression(ExprNode::BlockExpr(BlockExprNode {
            stmts: Vec::new(),
            expr: Some(sum),
            span: attr.span,
        }));

        let f = FunctionNode {
            name: ctx.intern("size"),
            parameters: vec![],
            generic_parameters: vec![],
            body: Some(block),
            return_type: Some(UncheckedType::Basic(IdentId::TYPE_FELT, attr.span)),
            qualifier: Qualifier {
                is_extern: false,
                is_const: false,
            },
            visibility: Visibility::Public,
            attrs: vec![],
            span: attr.span,
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
        attr: &AttrNode,
        ctx: &mut V,
    ) -> DefId {
        let offset_ident = ctx.intern("offset");
        let mut offset = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: None,
            segments: vec![],
            target: offset_ident,
            span: attr.span,
        }));
        let mut field_reads = IndexMap::new();

        for (field_name, field) in &struct_node.fields {
            let (_key, value) = self.generate_field_read(attr, field_name, &field.ty, offset, ctx);
            field_reads.insert(field_name.clone(), value);
            let node = BinaryNode {
                lhs: offset,
                operator: BinaryOperator::Add,
                rhs: self.generate_field_size(attr, &field.ty, ctx),
                span: attr.span,
            };
            offset = ctx.alloc_expression(ExprNode::Binary(node));
        }

        let value_node = ctx.alloc_expression(ExprNode::Value(ValueNode::Struct(
            struct_node.name,
            vec![],
            field_reads,
            attr.span,
        )));
        let block = ctx.alloc_expression(ExprNode::BlockExpr(BlockExprNode {
            stmts: Vec::new(),
            expr: Some(value_node),
            span: attr.span,
        }));
        let f = FunctionNode {
            name: ctx.intern("read"),
            parameters: vec![FunctionParameter::new(
                offset_ident,
                TypeQualifier::new(false),
                UncheckedType::Basic(IdentId::TYPE_FELT, attr.span),
                attr.span,
            )],
            generic_parameters: vec![],
            body: Some(block),
            return_type: Some(UncheckedType::Basic(IdentId::TYPE_SELF, attr.span)),
            qualifier: Qualifier {
                is_extern: false,
                is_const: false,
            },
            visibility: Visibility::Public,
            attrs: vec![],
            span: attr.span,
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
        attr: &AttrNode,
        ctx: &mut V,
    ) -> DefId {
        let offset_ident = ctx.intern("offset");
        let value_ident = ctx.intern("value");
        let mut offset = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: None,
            segments: vec![],
            target: offset_ident,
            span: attr.span,
        }));
        let mut field_writes = Vec::new();

        for (field_name, field) in &struct_node.fields {
            let stmt_id = self.generate_field_write(attr, field_name, &field.ty, offset, ctx);
            field_writes.push(stmt_id);
            let node = BinaryNode {
                lhs: offset,
                operator: BinaryOperator::Add,
                rhs: self.generate_field_size(attr, &field.ty, ctx),
                span: attr.span,
            };
            offset = ctx.alloc_expression(ExprNode::Binary(node));
        }
        let block = ctx.alloc_expression(ExprNode::BlockExpr(BlockExprNode {
            stmts: field_writes,
            expr: None,
            span: attr.span,
        }));

        let f = FunctionNode {
            name: ctx.intern("write"),
            parameters: vec![
                FunctionParameter::new(
                    offset_ident,
                    TypeQualifier::new(false),
                    UncheckedType::Basic(IdentId::TYPE_FELT, attr.span),
                    attr.span,
                ),
                FunctionParameter::new(
                    value_ident,
                    TypeQualifier::new(false),
                    UncheckedType::Basic(IdentId::TYPE_SELF, attr.span),
                    attr.span,
                ),
            ],
            generic_parameters: vec![],
            body: Some(block),
            return_type: None,
            qualifier: Qualifier {
                is_extern: false,
                is_const: false,
            },
            visibility: Visibility::Public,
            attrs: vec![],
            span: attr.span,
        };

        ctx.alloc_definition(DefinitionNode::Function(f))
    }

    fn generate_getter<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        attr: &AttrNode,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> DefId {
        let getter_name = format!("get_{}", ctx.ident(*field_name));
        let getter_ident = ctx.intern(getter_name.as_str());

        let read_ident = ctx.intern("read");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: Some(field_type.clone()),
            segments: vec![],
            target: read_ident,
            span: attr.span,
        }));

        let read_call = CallNode {
            callee: variable,
            generic_parameters: Vec::new(),
            args: vec![offset],
            span: attr.span,
        };
        let read_expr = ctx.alloc_expression(ExprNode::Call(read_call));

        let block = ctx.alloc_expression(ExprNode::BlockExpr(BlockExprNode {
            stmts: Vec::new(),
            expr: Some(read_expr),
            span: attr.span,
        }));

        let function = FunctionNode {
            name: getter_ident,
            parameters: vec![],
            generic_parameters: vec![],
            body: Some(block),
            return_type: Some(field_type.clone()),
            qualifier: Qualifier {
                is_extern: false,
                is_const: false,
            },
            visibility: Visibility::Public,
            attrs: vec![],
            span: attr.span,
        };

        ctx.alloc_definition(DefinitionNode::Function(function))
    }

    fn generate_setter<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        attr: &AttrNode,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> DefId {
        let setter_name = format!("set_{}", ctx.ident(*field_name));
        let setter_ident = ctx.intern(setter_name.as_str());

        let value_ident = ctx.intern("value");

        let write_ident = ctx.intern("write");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: Some(field_type.clone()),
            segments: vec![],
            target: write_ident,
            span: attr.span,
        }));

        let value = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: None,
            segments: vec![],
            target: value_ident,
            span: attr.span,
        }));

        let write_call = CallNode {
            callee: variable,
            generic_parameters: Vec::new(),
            args: vec![offset, value],
            span: attr.span,
        };
        let write_expr = ctx.alloc_expression(ExprNode::Call(write_call));

        let write_stmt = ctx.alloc_statement(StmtNode::Expression(write_expr));

        let block = ctx.alloc_expression(ExprNode::BlockExpr(BlockExprNode {
            stmts: vec![write_stmt],
            expr: None,
            span: attr.span,
        }));

        let function = FunctionNode {
            name: setter_ident,
            parameters: vec![FunctionParameter::new(
                value_ident,
                TypeQualifier::new(false),
                field_type.clone(),
                attr.span,
            )],
            generic_parameters: vec![],
            body: Some(block),
            return_type: None,
            qualifier: Qualifier {
                is_extern: false,
                is_const: false,
            },
            visibility: Visibility::Public,
            attrs: vec![],
            span: attr.span,
        };

        ctx.alloc_definition(DefinitionNode::Function(function))
    }

    fn generate_field_size<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        attr: &AttrNode,
        field_type: &UncheckedType,
        ctx: &mut V,
    ) -> ExprId {
        let size_ident = ctx.intern("size");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: Some(field_type.clone()),
            segments: vec![],
            target: size_ident,
            span: attr.span,
        }));
        let node = CallNode {
            callee: variable,
            generic_parameters: Vec::new(),
            args: Vec::new(),
            span: attr.span,
        };
        ctx.alloc_expression(ExprNode::Call(node))
    }

    fn generate_field_read<
        F: Clone + From<u32>,
        C,
        V: VisitorContext<F, C, Expr = ExprNode<F>, Stmt = StmtNode, Definition = DefinitionNode>,
    >(
        &self,
        attr: &AttrNode,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> (IdentId, ExprId) {
        let read_ident = ctx.intern("read");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: Some(field_type.clone()),
            segments: vec![],
            target: read_ident,
            span: attr.span,
        }));
        let node = CallNode {
            callee: variable,
            generic_parameters: Vec::new(),
            args: vec![offset],
            span: attr.span,
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
        attr: &AttrNode,
        field_name: &IdentId,
        field_type: &UncheckedType,
        offset: ExprId,
        ctx: &mut V,
    ) -> StmtId {
        let value_ident = ctx.intern("value");
        let write_ident = ctx.intern("write");
        let variable = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: Some(field_type.clone()),
            segments: vec![],
            target: write_ident,
            span: attr.span,
        }));
        let value = ctx.alloc_expression(ExprNode::Path(PathNode {
            root: None,
            segments: vec![],
            target: value_ident,
            span: attr.span,
        }));
        let field = ctx.alloc_expression(ExprNode::MemberAccess(MemberAccessNode {
            target: value,
            field: field_name.clone(),
            span: attr.span,
        }));
        let node = CallNode {
            callee: variable,
            generic_parameters: Vec::new(),
            args: vec![offset, field],
            span: attr.span,
        };
        let node_id = ctx.alloc_expression(ExprNode::Call(node));
        ctx.alloc_statement(StmtNode::Expression(node_id))
    }
}

impl<'a, F: Clone + From<u32> + 'static, C> AstVisitor<F, C> for StorageProcessor<'a> {
    type Context = DefaultVisitorContext<'a, F, C>;
    type ExprResult = ();
    type StmtResult = ();
    type Error = qed_common::Error;
    type Expr = ExprNode<F>;
    type Stmt = StmtNode;
    type Definition = DefinitionNode;
    type DefinitionResult = ();

    fn visit_use(
        &mut self,
        _def_id: DefId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_path(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_index_access(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_member_access(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_value(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_binary(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_unary(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_call(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_cast(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_while(
        &mut self,
        _node: StmtId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_assignment(
        &mut self,
        _node: StmtId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_variable(
        &mut self,
        _node: StmtId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_return(
        &mut self,
        _expr: StmtId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_impl(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        let defs: Vec<DefId> = ctx.definition(node).as_impl().unwrap().body.clone();
        for def_id in defs {
            self.visit_definition(def_id, ctx)?;
        }
        Ok(())
    }

    fn visit_trait(
        &mut self,
        _node: DefId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        Ok(())
    }

    fn visit_function(
        &mut self,
        _node: DefId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        Ok(())
    }

    fn visit_struct(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        let s: StructNode = ctx.definition(node).as_struct().unwrap().clone();
        let storage_trait_id = ctx.intern("Storage");
        let storage_attribute_id = ctx.intern("storage");

        for attr in &s.attrs {
            if attr.is_derive() && attr.properties.iter().any(|p| p == &storage_trait_id) {
                let impl_node = self.generate_storage_impl(&s, attr, ctx);
                let pos = ctx.node_id().as_def().unwrap().clone();
                ctx.insert_definition(
                    DefinitionNode::ImplTrait(impl_node),
                    InsertPosition::After(pos.into()),
                );
            }

            if attr.name == storage_attribute_id {
                let impl_node = self.generate_accessor_impl(&s, attr, ctx);
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
        _node: DefId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_intrinsic_expr(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_intrinsic_stmt(
        &mut self,
        _node: StmtId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_member_call(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_type_alias(
        &mut self,
        _node: DefId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        Ok(())
    }

    fn visit_const(
        &mut self,
        _node: DefId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        Ok(())
    }

    fn visit_for(
        &mut self,
        _node: StmtId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_match(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_lambda_function(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_impl_trait(
        &mut self,
        _node: DefId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        Ok(())
    }

    fn visit_if_expr(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_block_expr(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_tuple(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_tuple_access(
        &mut self,
        _node: ExprId,
        _ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }
}
