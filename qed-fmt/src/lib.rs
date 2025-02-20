use qed_ast::*;
use qedlang_core::dpn::ops::context_trait::{ContextFelt, DPNContext};
use std::fmt::Debug;

#[derive(Debug)]
pub struct Formatter<'a, F: Clone + From<u32>, C> {
    output: String,
    indent: usize,
    _marker: std::marker::PhantomData<(&'a (), F, C)>,
}

impl<'a, F: Clone + From<u32> + Debug, C> Formatter<'a, F, C> {
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
            UncheckedType::FunctionSignature(sig) => {
                let parameters = sig
                    .parameters
                    .iter()
                    .map(|p| {
                        format!(
                            "{}{}",
                            if p.0 { "mut " } else { "" },
                            self.visit_unchecked_type(&p.1, ctx)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "fn({}){}",
                    parameters,
                    if let Some(ref ret) = sig.return_type {
                        format!(" -> {}", self.visit_unchecked_type(&ret, ctx))
                    } else {
                        "".to_string()
                    }
                )
            }
        }
    }

    fn visit_generic_parameters(&self, generic_parameters: Vec<String>) -> String {
        if generic_parameters.is_empty() {
            "".to_string()
        } else {
            format!("#<{}>", generic_parameters.join(", "))
        }
    }

    pub fn get_output(&self) -> &str {
        &self.output
    }
}

impl<'a, F: ContextFelt + From<u32> + Debug + 'static, C: DPNContext<F>> AstVisitor<F, C>
    for Formatter<'a, F, C>
{
    type ExprResult = String;
    type StmtResult = String;
    type Context = DefaultVisitorContext<'a, F, C>;
    type Error = ();
    type Expr = ExprNode<F>;
    type Stmt = StmtNode;
    type Definition = DefinitionNode;
    type DefinitionResult = String;

    fn visit_use(&mut self, u: &UsePath, ctx: &mut Self::Context) -> Result<(), Self::Error> {
        let mut path = vec![ctx.ident(u.kind.clone().into()).to_string()];
        let segments = u
            .segments
            .iter()
            .map(|&s| ctx.ident(s).to_string())
            .collect::<Vec<_>>();
        path.extend(segments);
        let target = u
            .target
            .map(|t| ctx.ident(t).to_string())
            .unwrap_or("*".to_string());

        self.write_line(&format!("pub use {}::{};", path.join("::"), target));
        Ok(Default::default())
    }

    fn visit_path(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let node = ctx.expression(expr_id).as_path().unwrap();

        let mut path = node
            .root
            .map(|r| vec![ctx.ident(r).to_string()])
            .unwrap_or_default();
        path.extend_from_slice(
            &node
                .segments
                .iter()
                .map(|&s| ctx.ident(s).to_string())
                .collect::<Vec<String>>(),
        );
        path.extend_from_slice(&vec![ctx.ident(node.target).to_string()]);

        Ok(format!("{}", path.join("::")))
    }

    fn visit_index_access(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let &IndexAccessNode {
            target: value,
            index,
        } = ctx.expression(expr_id).as_index_access().unwrap();
        Ok(format!("{}[{}]", self.visit_expr(value, ctx)?, index))
    }

    fn visit_member_access(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let &MemberAccessNode {
            target: value,
            field,
        } = ctx.expression(expr_id).as_member_access().unwrap();
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
            ValueNode::Felt(f) => format!("{:?}", f),
            ValueNode::Bool(b) => format!("{:?}", b),
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
                let mut result = format!("new {}{} {{\n", name, generic_parameters);

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
            callee: variable,
            ref args,
            ref generic_parameters,
        } = ctx.expression(expr_id).as_call().unwrap();
        let generic_parameters = generic_parameters
            .iter()
            .map(|generic_parameter| self.visit_unchecked_type(&generic_parameter, ctx))
            .collect::<Vec<_>>();
        let args = args
            // TOOD: remove clone
            .clone()
            .iter()
            .map(|&arg| self.visit_expr(arg, ctx))
            .collect::<Result<Vec<_>, Self::Error>>()?
            .join(", ");
        Ok(format!(
            "{}{}({})",
            self.visit_expr(variable, ctx)?,
            self.visit_generic_parameters(generic_parameters),
            args
        ))
    }

    fn visit_member_call(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let &MemberCallNode {
            callee: variable,
            ref args,
            receiver,
            ref generic_parameters,
        } = ctx.expression(expr_id).as_member_call().unwrap();
        let generic_parameters = generic_parameters
            .iter()
            .map(|generic_parameter| self.visit_unchecked_type(&generic_parameter, ctx))
            .collect::<Vec<_>>();
        let args = args
            // TOOD: remove clone
            .clone()
            .iter()
            .map(|&arg| self.visit_expr(arg, ctx))
            .collect::<Result<Vec<_>, Self::Error>>()?
            .join(", ");
        Ok(format!(
            "{}{}({})",
            self.visit_expr(variable, ctx)?,
            self.visit_generic_parameters(generic_parameters),
            args
        ))
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
        let result = format!(
            "if {} {{",
            self.visit_expr(
                ctx.statement(stmt_id).as_if().unwrap().if_branch.predicate,
                ctx
            )?
        );
        self.write_line(&result);
        self.indent();
        self.visit_block(ctx.statement(stmt_id).as_if().unwrap().if_branch.body, ctx)?;
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
            self.visit_block(branch.body, ctx)?;
            self.dedent();
            self.write("}");
        }

        if let Some(else_branch) = ctx.statement(stmt_id).as_if().unwrap().else_branch {
            self.append_line(" else {");
            self.indent();
            self.visit_block(else_branch.clone(), ctx)?;
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
        let s = format!("while {} {{", self.visit_expr(predicate, ctx)?);
        self.write_line(&s);
        self.indent();
        self.visit_block(body, ctx)?;
        self.dedent();
        self.write_line("};");
        Ok(Default::default())
    }

    fn visit_block(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let BlockNode { stmts, uses } = ctx.statement(stmt_id).as_block().cloned().unwrap();
        // TODO: remove clone

        for &stmt in uses.iter() {
            let use_path = ctx.statement(stmt).as_use().cloned().unwrap();
            self.visit_use(&use_path, ctx)?;
        }

        for &stmt in stmts.iter() {
            let res = self.visit_stmt(stmt, ctx)?;
            if !res.is_empty() {
                self.write_line(&format!("{};", res));
            }
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
            "{} {} {};",
            self.visit_expr(variable, ctx)?,
            operator,
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

        let s = format!(
            "impl{} {}{}{} {{",
            if let Some(trait_name) = trait_name {
                format!(" {} for", &ctx.ident(trait_name.clone()))
            } else {
                "".to_string()
            },
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
            qualifier,
            visibility,
            attrs,
        } = ctx.definition(def_id).as_function().unwrap();
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
        let s = format!(
            "{}{}fn {}{}({}){} {{",
            if visibility.is_public() { "pub " } else { "" },
            if qualifier.is_extern { "extern " } else { "" },
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
        self.visit_block(body.unwrap(), ctx)?;
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
            visibility,
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
            "{}struct {}{} {{",
            if visibility.is_public() { "pub " } else { "" },
            &ctx.ident(name.clone()),
            self.visit_generic_parameters(
                generic_parameters
                    .iter()
                    .map(|p| ctx.ident(p.clone()).to_string())
                    .collect::<Vec<_>>()
            )
        ));
        self.indent();
        for (field, (value, visibility)) in fields {
            let s = format!(
                "{}{}: {},",
                if visibility.is_public() { "pub " } else { "" },
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
            visibility,
        } = ctx.definition(def_id).as_enum().unwrap();
        self.write_line(&format!(
            "{}enum {}{} {{",
            if visibility.is_public() { "pub " } else { "" },
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
                    for (field, (ty, _visibility)) in fields {
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
            visibility,
        } = ctx.definition(def_id).as_trait().unwrap();
        self.write_line(&format!(
            "{}trait {}{} {{",
            if visibility.is_public() { "pub " } else { "" },
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
            let func = ctx.definition(func.clone()).as_function().unwrap();
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
                "{}{}fn {}({}){};",
                if func.visibility.is_public() {
                    "pub "
                } else {
                    ""
                },
                if func.qualifier.is_extern {
                    "extern "
                } else {
                    ""
                },
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

    fn visit_intrinsic_expr(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let node = ctx.expression(node).as_intrinsic().cloned().unwrap();
        match node {
            IntrinsicExprNode::GetUserId => Ok("__ctx_get_user_id()".to_string()),
            IntrinsicExprNode::GetContractId => Ok("__ctx_get_contract_id()".to_string()),
            IntrinsicExprNode::GetLastNonce => Ok("__ctx_get_last_nonce()".to_string()),
            IntrinsicExprNode::GetCheckpointId => Ok("__ctx_get_checkpoint_id()".to_string()),
            IntrinsicExprNode::GetUserPublicKeyHash => {
                Ok("__ctx_get_user_public_key_hash()".to_string())
            }
            IntrinsicExprNode::GetStateHashAt { slot_index } => Ok(format!(
                "__ctx_get_state_hash_at({})",
                self.visit_expr(slot_index.clone(), ctx)?
            )),
            IntrinsicExprNode::GetOtherContractStateHashAt {
                contract_state_tree_height,
                contract_id,
                slot_index,
            } => Ok(format!(
                "__ctx_get_other_contract_state_hash_at({}, {}, {})",
                self.visit_expr(contract_state_tree_height.clone(), ctx)?,
                self.visit_expr(contract_id.clone(), ctx)?,
                self.visit_expr(slot_index.clone(), ctx)?
            )),
            IntrinsicExprNode::GetOtherUserContractStateHashAt {
                contract_state_tree_height,
                user_id,
                contract_id,
                slot_index,
            } => Ok(format!(
                "__ctx_get_other_user_contract_state_hash_at({}, {}, {}, {})",
                self.visit_expr(contract_state_tree_height.clone(), ctx)?,
                self.visit_expr(user_id.clone(), ctx)?,
                self.visit_expr(contract_id.clone(), ctx)?,
                self.visit_expr(slot_index.clone(), ctx)?
            )),
            IntrinsicExprNode::CSetStateHashAt {
                slot_index,
                new_value,
            } => Ok(format!(
                "__ctx_cset_state_hash_at({}, {})",
                self.visit_expr(slot_index.clone(), ctx)?,
                self.visit_expr(new_value.clone(), ctx)?
            )),
            IntrinsicExprNode::Read { offset } => {
                Ok(format!("__storage_read({})", self.visit_expr(offset, ctx)?))
            }
            IntrinsicExprNode::Write { offset, value } => Ok(format!(
                "__storage_write({}, {})",
                self.visit_expr(offset, ctx)?,
                self.visit_expr(value, ctx)?
            )),
            IntrinsicExprNode::Hash { data } => {
                Ok(format!("hash({})", self.visit_expr(data, ctx)?,))
            }
        }
    }

    fn visit_intrinsic_stmt(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let node = ctx.statement(node).as_intrinsic().cloned().unwrap();
        match node {
            IntrinsicStmtNode::Assert { left, message } => {
                let expr = self.visit_expr(left, ctx)?;
                self.write_line(&format!(
                    "assert({}, \"{}\")",
                    expr,
                    message.unwrap_or_default()
                ))
            }
            IntrinsicStmtNode::AssertEq {
                left,
                right,
                message,
            } => {
                let left = self.visit_expr(left, ctx)?;
                let right = self.visit_expr(right, ctx)?;
                self.write_line(&format!(
                    "assert_eq({}, {}, \"{}\")",
                    left,
                    right,
                    message.unwrap_or_default()
                ))
            }
        }
        Ok(Default::default())
    }

    fn visit_module(
        &mut self,
        module_id: ModuleId,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        ctx.push_node_id(NodeId::from(module_id));

        let module = ctx.module(module_id).clone();

        let visibility_string = match module.visibility {
            Visibility::Public => "pub ",
            Visibility::Private => "",
        };

        self.write_line(&format!(
            "{}mod {} {{",
            visibility_string,
            &ctx.ident(module.name)
        ));
        self.indent();

        // TODO: remove clone
        for u in &module.uses {
            self.visit_use(u, ctx)?;
        }

        for &child_module in ctx.program().modules.nodes().clone()[module_id].children() {
            // let child_module = ctx.module(child_module).clone();
            self.visit_module(child_module, ctx)?;
        }

        // TODO: remove clone
        for &definition in &module.definitions {
            self.visit_definition(definition, ctx)?;
        }

        self.dedent();
        self.write_line(&format!("}}"));

        ctx.pop_node_id();
        Ok(())
    }

    fn visit_program(&mut self, ctx: &mut Self::Context) -> Result<(), Self::Error> {
        self.visit_module(ModuleId::root(), ctx)?;
        Ok(())
    }

    fn visit_type_alias(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let node = ctx.definition(node).as_type_alias().cloned().unwrap();
        self.write_line(&format!(
            "type {} = {};",
            ctx.ident(node.name),
            self.visit_unchecked_type(&node.ty, ctx)
        ));
        Ok(Default::default())
    }

    fn visit_const(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        let node = ctx.definition(node).as_const().cloned().unwrap();
        let value = self.visit_expr(node.value, ctx)?;
        self.write_line(&format!(
            "{}const {}:{} = {};",
            if node.visibility.is_public() {
                "pub "
            } else {
                ""
            },
            ctx.ident(node.name),
            self.visit_unchecked_type(&node.ty, ctx),
            value
        ));
        Ok(Default::default())
    }
}
