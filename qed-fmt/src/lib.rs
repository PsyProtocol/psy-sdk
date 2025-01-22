use qed_ast::*;
use qed_builder::{Context, ContextFelt};
use qed_common::Graph;
use qed_parser::Parser;
use std::fmt::{Display, Write};

pub struct FormatterContext<'a, F: Clone + From<u32>, C> {
    path_stack: Vec<NodeId>,
    program: &'a Program<F>,
    _marker: std::marker::PhantomData<(F, C)>,
}

impl<'a, F: Clone + From<u32>, C> FormatterContext<'a, F, C> {
    pub fn new(program: &'a Program<F>) -> Self {
        FormatterContext {
            path_stack: vec![],
            program,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'a, F: Clone + From<u32>, C> VisitorContext<F, C> for FormatterContext<'a, F, C> {
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
        unreachable!()
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
        unimplemented!()
    }

    fn alloc_expression(&mut self, expr: ExprNode<F>) -> ExprId {
        unimplemented!()
    }

    fn alloc_statement(&mut self, stmt: StmtNode<F>) -> StmtId {
        unimplemented!()
    }

    fn alloc_definition(&mut self, definition: DefinitionNode) -> DefId {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct Formatter<'a, F: Clone + From<u32>, C> {
    output: String,
    indent: usize,
    _marker: std::marker::PhantomData<(&'a (), F, C)>,
}

impl<'a, F: Clone + From<u32> + Display, C> Formatter<'a, F, C> {
    pub fn new() -> Self {
        Formatter {
            output: String::new(),
            indent: 0,
            _marker: std::marker::PhantomData,
        }
    }

    fn indent(&mut self) {
        self.indent += 1;
    }

    fn dedent(&mut self) {
        if self.indent > 0 {
            self.indent -= 1;
        }
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
    }

    fn read_indent(&self, extra: usize) -> String {
        let mut result = String::new();
        for _ in 0..self.indent + extra {
            result.push_str("  ");
        }
        result
    }

    fn append(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn append_line(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn write(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
    }

    fn write_line(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn visit_unchecked_type(
        &self,
        node: &UncheckedType,
        ctx: &impl VisitorContext<F, C>,
    ) -> String {
        match node {
            UncheckedType::Basic(name) => ctx.ident(name.clone()).to_string(),
            UncheckedType::Generic(name, generic_parameters) => format!(
                "{}{}",
                &ctx.ident(name.clone()),
                self.visit_generic_parameters(
                    generic_parameters
                        .into_iter()
                        .map(|ty| self.visit_unchecked_type(ty, ctx))
                        .collect::<Vec<_>>()
                )
            ),
            UncheckedType::Array(ty, size) => {
                format!("[{};{}]", self.visit_unchecked_type(ty, ctx), size)
            }
            UncheckedType::Unknown => "unknown".to_string(),
        }
    }

    fn visit_generic_parameters(&self, generic_parameters: Vec<String>) -> String {
        if generic_parameters.is_empty() {
            "".to_string()
        } else {
            format!("<{}>", generic_parameters.join(", "))
        }
    }

    pub fn get_output(&self) -> &str {
        &self.output
    }
}

impl<'a, F: ContextFelt + From<u32> + Display + 'static, C: Context<F>> AstVisitor<F, C>
    for Formatter<'a, F, C>
{
    type ExprResult = String;
    type StmtResult = String;
    type Context = FormatterContext<'a, F, C>;
    type Error = ();

    fn visit_use(&mut self, u: &UsePath, ctx: &mut Self::Context) -> Result<(), Self::Error> {
        let mut path = vec![ctx.ident(u.kind.clone().into()).to_string()];
        let mut segments = u
            .segments
            .iter()
            .map(|&s| ctx.ident(s).to_string())
            .collect::<Vec<_>>();
        path.extend(segments);
        let target = u
            .target
            .map(|t| ctx.ident(t).to_string())
            .unwrap_or("*".to_string());

        self.write_line(&format!("use {}::{};", path.join("::"), target));
        Ok(Default::default())
    }

    fn visit_path(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let node = ctx.expression(expr_id).as_path().unwrap();
        Ok(ctx.ident(node.target).to_string())
    }

    fn visit_index_access(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let &IndexAccessNode { value, index } = ctx.expression(expr_id).as_index_access().unwrap();
        Ok(format!("{}[{}]", self.visit_expr(value, ctx)?, index))
    }

    fn visit_member_access(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let &MemberAccessNode { value, field } =
            ctx.expression(expr_id).as_member_access().unwrap();
        Ok(format!(
            "{}.{}",
            self.visit_expr(value, ctx)?,
            ctx.ident(field)
        ))
    }

    fn visit_value(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let node = ctx.expression(expr_id).as_value().unwrap().clone();
        Ok(match node {
            ValueNode::Felt(f) => f.to_string(),
            ValueNode::Bool(b) => b.to_string(),
            ValueNode::Array(_, values) => format!(
                "[{}]",
                values
                    // TODO: remove clone
                    .clone()
                    .into_iter()
                    .map(|v| self.visit_expr(v, ctx))
                    .collect::<Result<Vec<_>, Self::Error>>()?
                    .join(", ")
            ),
            ValueNode::Struct(name, generic_parameters, field_values) => {
                let name = ctx.ident(name.clone());
                let generic_parameters = self.visit_generic_parameters(
                    generic_parameters
                        .iter()
                        .map(|p| self.visit_unchecked_type(p, ctx))
                        .collect::<Vec<_>>(),
                );
                let mut result = format!("{}{} {{\n", name, generic_parameters);

                for (field, value) in field_values {
                    result.push_str(&self.read_indent(1));
                    result.push_str(&format!(
                        "{}: {},\n",
                        ctx.ident(field).to_owned(),
                        self.visit_expr(value, ctx)?
                    ));
                }

                result.push_str(&self.read_indent(0));
                result.push_str("}");
                result
            }
        })
    }

    fn visit_binary(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let &BinaryNode { lhs, operator, rhs } = ctx.expression(expr_id).as_binary().unwrap();
        Ok(format!(
            "{} {} {}",
            self.visit_expr(lhs, ctx)?,
            operator,
            self.visit_expr(rhs, ctx)?
        ))
    }

    fn visit_unary(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let &UnaryNode { operator, rhs } = ctx.expression(expr_id).as_unary().unwrap();
        Ok(format!("{}{}", operator, self.visit_expr(rhs, ctx)?))
    }

    fn visit_call(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let &CallNode {
            variable,
            ref args,
            receiver,
            ref generic_parameters,
        } = ctx.expression(expr_id).as_call().unwrap();
        let args = args
            // TOOD: remove clone
            .clone()
            .iter()
            .map(|&arg| self.visit_expr(arg, ctx))
            .collect::<Result<Vec<_>, Self::Error>>()?
            .join(", ");
        Ok(format!("{}({})", self.visit_expr(variable, ctx)?, args))
    }

    fn visit_cast(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let &CastNode {
            value,
            ref target_type,
        } = ctx.expression(expr_id).as_cast().unwrap();
        // TODO: remove clone
        let target_type = target_type.clone();
        Ok(format!(
            "({} as {})",
            self.visit_expr(value, ctx)?,
            self.visit_unchecked_type(&target_type, ctx)
        ))
    }

    fn visit_if(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let mut result = format!(
            "if {} {{",
            self.visit_expr(
                ctx.statement(stmt_id).as_if().unwrap().if_branch.predicate,
                ctx
            )?
        );
        self.write_line(&result);
        self.indent();
        self.visit_block(ctx.statement(stmt_id).as_if().unwrap().if_branch.body, ctx);
        self.dedent();
        self.write("}");

        // TODO: remove clone
        for branch in ctx
            .statement(stmt_id)
            .as_if()
            .unwrap()
            .elseif_branch
            .clone()
            .into_iter()
        {
            let s = format!(" else if {} {{", self.visit_expr(branch.predicate, ctx)?);
            self.append_line(&s);
            self.indent();
            self.visit_block(branch.body, ctx);
            self.dedent();
            self.write("}");
        }

        if let Some(else_branch) = ctx.statement(stmt_id).as_if().unwrap().else_branch {
            self.append_line(" else {");
            self.indent();
            self.visit_block(else_branch.clone(), ctx);
            self.dedent();
            self.write("}");
        }

        self.append_line(";");
        Ok(Default::default())
    }

    fn visit_while(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let &WhileNode { predicate, body } = ctx.statement(stmt_id).as_while().unwrap();
        let mut s = format!("while {} {{", self.visit_expr(predicate, ctx)?);
        self.write_line(&s);
        self.indent();
        self.visit_block(body, ctx);
        self.dedent();
        self.write_line("};");
        Ok(Default::default())
    }

    fn visit_block(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let BlockNode { stmts } = ctx.statement(stmt_id).as_block().unwrap();
        // TODO: remove clone
        for stmt in stmts.clone() {
            self.visit_stmt(stmt, ctx)?;
        }
        Ok(Default::default())
    }

    fn visit_assignment(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let &AssignmentNode {
            variable,
            operator,
            value,
        } = ctx.statement(stmt_id).as_assignment().unwrap();
        let s = format!(
            "{} = {};",
            self.visit_expr(variable, ctx)?,
            self.visit_expr(value, ctx)?
        );
        self.write_line(&s);
        Ok(Default::default())
    }

    fn visit_variable(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let s = format!(
            "{}{} {}: {} = {};",
            if ctx.statement(stmt_id).as_variable().unwrap().cnst {
                "const"
            } else {
                "let"
            },
            if ctx.statement(stmt_id).as_variable().unwrap().mutable {
                " mut"
            } else {
                ""
            },
            // TODO: remove to_owned
            ctx.ident(ctx.statement(stmt_id).as_variable().unwrap().name)
                .to_owned(),
            self.visit_unchecked_type(
                &ctx.statement(stmt_id).as_variable().unwrap().ty.clone(),
                ctx
            ),
            self.visit_expr(ctx.statement(stmt_id).as_variable().unwrap().value, ctx)?
        );
        self.write_line(&s);
        Ok(Default::default())
    }

    fn visit_return(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let ReturnNode(expr_id) = ctx.statement(stmt_id).as_return().unwrap();
        let s = format!(
            "return{};",
            if let Some(ret) = expr_id {
                format!(" {}", self.visit_expr(ret.clone(), ctx)?)
            } else {
                "".to_string()
            }
        );
        self.write_line(&s);
        Ok(Default::default())
    }

    fn visit_impl(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let ImplNode {
            generic_parameters,
            trait_name,
            ty,
            body,
        } = ctx.definition(def_id).as_impl().unwrap();
        let generic_parameters = generic_parameters
            .iter()
            .map(|&generic_parameter| ctx.ident(generic_parameter).to_string())
            .collect::<Vec<_>>();
        let generic_parameters = self.visit_generic_parameters(generic_parameters);

        let mut s = format!(
            "impl{} {}{} {{",
            generic_parameters,
            &ctx.ident(ty.clone()),
            generic_parameters
        );
        self.write_line(&s);
        self.indent();
        // TODO: remove clone
        for func in body.clone() {
            self.visit_function(func, ctx)?;
        }
        self.dedent();
        self.write_line("}");
        Ok(Default::default())
    }

    fn visit_function(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let FunctionNode {
            name,
            parameters,
            generic_parameters,
            body,
            return_type,
            is_extern,
        } = ctx.definition(def_id).as_function().unwrap();
        let parameters = parameters
            .iter()
            .map(|p| {
                format!(
                    "{}{}: {}",
                    if p.1 { "mut " } else { "" },
                    &ctx.ident(p.0),
                    self.visit_unchecked_type(&p.2, ctx)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let generic_parameters = generic_parameters
            .iter()
            .map(|x| ctx.ident(x.clone()).to_string())
            .collect::<Vec<_>>();
        let mut s = format!(
            "fn {}{}({}){} {{",
            ctx.ident(name.clone()),
            self.visit_generic_parameters(generic_parameters),
            parameters,
            if let Some(ref ret) = return_type {
                format!(" -> {}", self.visit_unchecked_type(&ret, ctx))
            } else {
                "".to_string()
            }
        );
        self.write_line(&s);
        self.indent();
        self.visit_block(body.unwrap(), ctx);
        self.dedent();
        self.write_line("}");
        Ok(Default::default())
    }

    fn visit_struct(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let StructNode {
            name,
            fields,
            generic_parameters,
            attrs,
        } = ctx.definition(def_id).as_struct().unwrap();
        for attr in attrs {
            if !attr.properties.is_empty() {
                self.write_line(&format!(
                    "#[{}({})]",
                    ctx.ident(attr.name),
                    attr.properties
                        .iter()
                        .map(|p| ctx.ident(p.clone()).to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            } else {
                self.write_line(&format!("#[{}]", ctx.ident(attr.name),));
            }
        }
        self.write_line(&format!(
            "struct {}{} {{",
            &ctx.ident(name.clone()),
            self.visit_generic_parameters(
                generic_parameters
                    .iter()
                    .map(|p| ctx.ident(p.clone()).to_string())
                    .collect::<Vec<_>>()
            )
        ));
        self.indent();
        for (field, value) in fields {
            let s = format!(
                "{}: {},",
                ctx.ident(field.clone()),
                self.visit_unchecked_type(&value, ctx)
            );
            self.write_line(&s);
        }
        self.dedent();
        self.write_line("}");
        Ok(Default::default())
    }

    fn visit_enum(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let EnumNode {
            name,
            generic_parameters,
            variants,
        } = ctx.definition(def_id).as_enum().unwrap();
        self.write_line(&format!(
            "enum {}{} {{",
            &ctx.ident(name.clone()),
            self.visit_generic_parameters(
                generic_parameters
                    .iter()
                    .map(|p| ctx.ident(p.clone()).to_string())
                    .collect::<Vec<_>>()
            )
        ));
        self.indent();
        for variant in variants {
            match variant {
                EnumVariant::Basic(ident_id) => {
                    let s = format!("{}", ctx.ident(ident_id.clone()));
                    self.write_line(&s);
                }
                EnumVariant::Tuple(ident_id, types) => {
                    let types = types
                        .iter()
                        .map(|x| self.visit_unchecked_type(x, ctx))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let s = format!("{}({})", ctx.ident(ident_id.clone()), types);
                    self.write_line(&s);
                }
                EnumVariant::Struct(ident_id, fields) => {
                    self.write_line(&format!("{} {{", ctx.ident(ident_id.clone())));
                    self.indent();
                    for (field, ty) in fields {
                        self.write_line(&format!(
                            "{}: {},",
                            ctx.ident(field.clone()),
                            self.visit_unchecked_type(&ty, ctx)
                        ));
                    }
                    self.dedent();
                    self.write_line("}");
                }
            }
        }
        self.dedent();
        self.write_line("}");
        Ok(Default::default())
    }

    fn visit_trait(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let TraitNode {
            name,
            generic_parameters,
            body,
        } = ctx.definition(def_id).as_trait().unwrap();
        self.write_line(&format!(
            "trait {}{} {{",
            &ctx.ident(name.clone()),
            self.visit_generic_parameters(
                generic_parameters
                    .iter()
                    .map(|p| ctx.ident(p.clone()).to_string())
                    .collect::<Vec<_>>()
            )
        ));
        self.indent();
        for func in body {
            let parameters = func
                .parameters
                .iter()
                .map(|p| {
                    format!(
                        "{}{}: {}",
                        if p.1 { "mut " } else { "" },
                        &ctx.ident(p.0),
                        self.visit_unchecked_type(&p.2, ctx)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let s = format!(
                "fn {}({}){};",
                ctx.ident(func.name.clone()),
                parameters,
                if let Some(ref ret) = func.return_type {
                    format!(" -> {}", self.visit_unchecked_type(&ret, ctx))
                } else {
                    "".to_string()
                }
            );
            self.write_line(&s);
        }
        self.dedent();
        self.write_line("}");
        Ok(Default::default())
    }
}
