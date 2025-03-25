use qed_ast::BlockExprNode;
use qed_ast::IfExprNode;
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

    #[allow(dead_code)]
    fn append_line(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
    }

    #[allow(dead_code)]
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
            UncheckedType::Basic(name) => ctx.ident(name).to_string(),
            UncheckedType::Generic(name, generic_parameters, _) => format!(
                "{}{}",
                &ctx.ident(name),
                self.visit_generic_parameters(
                    generic_parameters
                        .into_iter()
                        .map(|ty| self.visit_unchecked_type(ty, ctx))
                        .collect::<Vec<_>>()
                )
            ),
            UncheckedType::Array(ty, size, _) => {
                format!("[{};{}]", self.visit_unchecked_type(ty, ctx), size)
            }
            UncheckedType::Tuple(tys, _) => format!(
                "({})",
                tys.iter()
                    .map(|ty| self.visit_unchecked_type(ty, ctx))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            UncheckedType::Unknown => "unknown".to_string(),
            UncheckedType::FunctionSignature(sig, _) => {
                let parameters = sig
                    .parameters
                    .iter()
                    .map(|p| format!("{}", self.visit_unchecked_type(&p, ctx)))
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
    type Error = qed_common::Error;
    type Expr = ExprNode<F>;
    type Stmt = StmtNode;
    type Definition = DefinitionNode;
    type DefinitionResult = String;

    fn visit_use(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        let u = ctx.definition(def_id).as_use().cloned().unwrap();
        let mut path = vec![ctx.ident(u.kind).to_string()];
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

        self.write_line(&format!(
            "{}use {}::{};",
            if u.visibility.is_public() { "pub " } else { "" },
            path.join("::"),
            target
        ));
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
            .as_ref()
            .map(|r| vec![self.visit_unchecked_type(r, ctx)])
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
            location: ref _location,
        } = ctx.expression(expr_id).as_index_access().unwrap();
        Ok(format!(
            "{}[{}]",
            self.visit_expr(value, ctx)?,
            self.visit_expr(index, ctx)?
        ))
    }

    fn visit_member_access(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let &MemberAccessNode {
            target: value,
            field,
            location: ref _location,
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
            ValueNode::Felt(f, _) => format!("{:?}", f),
            ValueNode::Bool(b, _) => format!("{:?}", b),
            ValueNode::U32(u, _) => format!("{:?}", u),
            ValueNode::Array(_, values, _) => format!(
                "[{}]",
                values
                    // TODO: remove clone
                    .clone()
                    .into_iter()
                    .map(|v| self.visit_expr(v, ctx))
                    .collect::<Result<Vec<_>, Self::Error>>()?
                    .join(", ")
            ),
            ValueNode::Struct(name, generic_parameters, field_values, _location) => {
                let name = ctx.ident(name);
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
        let &BinaryNode {
            lhs,
            operator,
            rhs,
            location: ref _location,
        } = ctx.expression(expr_id).as_binary().unwrap();
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
        let &UnaryNode {
            operator,
            rhs,
            location: ref _location,
        } = ctx.expression(expr_id).as_unary().unwrap();
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
            location: ref _location,
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
            ref generic_parameters,
            ..
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
            location: ref _location,
        } = ctx.expression(expr_id).as_cast().unwrap();
        // TODO: remove clone
        let target_type = target_type.clone();
        Ok(format!(
            "({} as {})",
            self.visit_expr(value, ctx)?,
            self.visit_unchecked_type(&target_type, ctx)
        ))
    }

    fn visit_while(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let &WhileNode {
            predicate,
            body,
            location: ref _location,
        } = ctx.statement(stmt_id).as_while().unwrap();
        let s = format!("while {} ", self.visit_expr(predicate, ctx)?);
        let block = self.visit_block_expr(body, ctx)?;
        self.write_line(&format!("{}{}", s, block));
        Ok(Default::default())
    }

    fn visit_assignment(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let &AssignmentNode {
            target,
            operator,
            value,
            location: ref _location,
        } = ctx.statement(stmt_id).as_assignment().unwrap();
        let s = format!(
            "{} {} {};",
            self.visit_expr(target, ctx)?,
            operator,
            self.visit_expr(value, ctx)?
        );
        Ok(s)
    }

    fn visit_variable(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let s = format!(
            "let{} {}: {} = {};",
            if ctx
                .statement(stmt_id)
                .as_variable()
                .unwrap()
                .qualifier
                .is_mutable
            {
                " mut"
            } else {
                ""
            },
            // TODO: remove to_owned
            ctx.ident(ctx.statement(stmt_id).as_variable().unwrap().name.id)
                .to_owned(),
            self.visit_unchecked_type(
                &ctx.statement(stmt_id).as_variable().unwrap().ty.clone(),
                ctx
            ),
            self.visit_expr(ctx.statement(stmt_id).as_variable().unwrap().value, ctx)?
        );
        Ok(s)
    }

    fn visit_return(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let ReturnNode {
            expr_id,
            location: ref _location,
        } = ctx.statement(stmt_id).as_return().unwrap();
        let s = format!(
            "return{};",
            if let Some(ret) = expr_id {
                format!(" {}", self.visit_expr(ret.clone(), ctx)?)
            } else {
                "".to_string()
            }
        );
        Ok(s)
    }

    fn visit_impl(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let ImplNode {
            generic_parameters,
            ty,
            body,
            location: _location,
        } = ctx.definition(def_id).as_impl().unwrap();
        let generic_parameters = generic_parameters
            .iter()
            .map(|generic_parameter| ctx.ident(generic_parameter.name).to_string())
            .collect::<Vec<_>>();
        let generic_parameters = self.visit_generic_parameters(generic_parameters);

        let s = format!(
            "impl{} {} {{",
            generic_parameters,
            self.visit_unchecked_type(&ty, ctx),
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
    ) -> Result<Self::DefinitionResult, Self::Error> {
        let FunctionNode {
            name,
            parameters,
            generic_parameters,
            body,
            return_type,
            qualifier,
            visibility,
            attrs,
            location: _location,
        } = ctx.definition(def_id).as_function().unwrap();
        for attr in attrs {
            if !attr.properties.is_empty() {
                self.write_line(&format!(
                    "#[{}({})]",
                    ctx.ident(attr.name),
                    attr.properties
                        .iter()
                        .map(|p| ctx.ident(p).to_string())
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
                    if p.qualifier.is_mutable { "mut " } else { "" },
                    &ctx.ident(p.name),
                    self.visit_unchecked_type(&p.ty, ctx)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let generic_parameters = generic_parameters
            .iter()
            .map(|x| ctx.ident(x.name.clone()).to_string())
            .collect::<Vec<_>>();
        let s = format!(
            "{}{}fn {}{}({}){} ",
            if visibility.is_public() { "pub " } else { "" },
            if qualifier.is_extern { "extern " } else { "" },
            ctx.ident(name),
            self.visit_generic_parameters(generic_parameters),
            parameters,
            if let Some(ref ret) = return_type {
                format!(" -> {}", self.visit_unchecked_type(&ret, ctx))
            } else {
                "".to_string()
            }
        );
        let block = match body {
            Some(body) => self.visit_block_expr(body.clone(), ctx)?,
            None => "{ }".to_string(),
        };
        self.write_line(&format!("{}{}", s, block));
        Ok(Default::default())
    }

    fn visit_struct(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        let StructNode {
            name,
            fields,
            generic_parameters,
            attrs,
            visibility,
            location: _location,
        } = ctx.definition(def_id).as_struct().unwrap();
        for attr in attrs {
            if !attr.properties.is_empty() {
                self.write_line(&format!(
                    "#[{}({})]",
                    ctx.ident(attr.name),
                    attr.properties
                        .iter()
                        .map(|p| ctx.ident(p).to_string())
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
            &ctx.ident(name),
            self.visit_generic_parameters(
                generic_parameters
                    .iter()
                    .map(|p| ctx.ident(p.name.clone()).to_string())
                    .collect::<Vec<_>>()
            )
        ));
        self.indent();
        for (field_name, field) in fields {
            let s = format!(
                "{}{}: {},",
                if visibility.is_public() { "pub " } else { "" },
                ctx.ident(field_name),
                self.visit_unchecked_type(&field.ty, ctx)
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
    ) -> Result<Self::DefinitionResult, Self::Error> {
        let EnumNode {
            name,
            generic_parameters,
            variants,
            visibility,
            location: _location,
        } = ctx.definition(def_id).as_enum().unwrap();
        self.write_line(&format!(
            "{}enum {}{} {{",
            if visibility.is_public() { "pub " } else { "" },
            &ctx.ident(name),
            self.visit_generic_parameters(
                generic_parameters
                    .iter()
                    .map(|p| ctx.ident(p.name.clone()).to_string())
                    .collect::<Vec<_>>()
            )
        ));
        self.indent();
        for variant in variants {
            match variant {
                EnumVariant::Basic(ident_id) => {
                    let s = format!("{},", ctx.ident(ident_id));
                    self.write_line(&s);
                }
                EnumVariant::Tuple(ident_id, types) => {
                    let types = types
                        .iter()
                        .map(|x| self.visit_unchecked_type(x, ctx))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let s = format!("{}({}),", ctx.ident(ident_id), types);
                    self.write_line(&s);
                }
                EnumVariant::Struct(ident_id, fields) => {
                    self.write_line(&format!("{} {{", ctx.ident(ident_id)));
                    self.indent();
                    for (field_name, field) in fields {
                        self.write_line(&format!(
                            "{}: {},",
                            ctx.ident(field_name),
                            self.visit_unchecked_type(&field.ty, ctx)
                        ));
                    }
                    self.dedent();
                    self.write_line("},");
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
    ) -> Result<Self::DefinitionResult, Self::Error> {
        let TraitNode {
            name,
            generic_parameters,
            body,
            visibility,
            location: _location,
        } = ctx.definition(def_id).as_trait().unwrap();
        self.write_line(&format!(
            "{}trait {}{} {{",
            if visibility.is_public() { "pub " } else { "" },
            &ctx.ident(name),
            self.visit_generic_parameters(
                generic_parameters
                    .iter()
                    .map(|p| ctx.ident(p.name.clone()).to_string())
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
                        if p.qualifier.is_mutable { "mut " } else { "" },
                        &ctx.ident(p.name),
                        self.visit_unchecked_type(&p.ty, ctx)
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
                ctx.ident(func.name),
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
            IntrinsicExprNode::GetUserId { .. } => Ok("__ctx_get_user_id()".to_string()),
            IntrinsicExprNode::GetContractId { .. } => Ok("__ctx_get_contract_id()".to_string()),
            IntrinsicExprNode::GetLastNonce { .. } => Ok("__ctx_get_last_nonce()".to_string()),
            IntrinsicExprNode::GetCheckpointId { .. } => {
                Ok("__ctx_get_checkpoint_id()".to_string())
            }
            IntrinsicExprNode::GetUserPublicKeyHash { .. } => {
                Ok("__ctx_get_user_public_key_hash()".to_string())
            }
            IntrinsicExprNode::GetStateHashAt { slot_index, .. } => Ok(format!(
                "__ctx_get_state_hash_at({})",
                self.visit_expr(slot_index.clone(), ctx)?
            )),
            IntrinsicExprNode::GetOtherContractStateHashAt {
                contract_state_tree_height,
                contract_id,
                slot_index,
                ..
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
                ..
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
                ..
            } => Ok(format!(
                "__ctx_cset_state_hash_at({}, {})",
                self.visit_expr(slot_index.clone(), ctx)?,
                self.visit_expr(new_value.clone(), ctx)?
            )),
            IntrinsicExprNode::Transmute {
                data, target_type, ..
            } => Ok(format!(
                "__mem_transmute({}, {})",
                self.visit_expr(data, ctx)?,
                self.visit_unchecked_type(&target_type, ctx)
            )),
            IntrinsicExprNode::Read { offset, .. } => {
                Ok(format!("__storage_read({})", self.visit_expr(offset, ctx)?))
            }
            IntrinsicExprNode::ReadRange { offset, length, .. } => Ok(format!(
                "__storage_read_range({},{})",
                self.visit_expr(offset, ctx)?,
                self.visit_expr(length, ctx)?
            )),
            IntrinsicExprNode::Write { offset, value, .. } => Ok(format!(
                "__storage_write({}, {})",
                self.visit_expr(offset, ctx)?,
                self.visit_expr(value, ctx)?
            )),
            IntrinsicExprNode::WriteRange { offset, values, .. } => Ok(format!(
                "__storage_write_range({}, {})",
                self.visit_expr(offset, ctx)?,
                self.visit_expr(values, ctx)?
            )),
            IntrinsicExprNode::Hash { data, .. } => {
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
        let s = match node {
            IntrinsicStmtNode::Assert {
                left,
                message,
                location: _location,
            } => {
                let expr = self.visit_expr(left, ctx)?;
                format!("assert({}, \"{}\")", expr, message.unwrap_or_default())
            }
            IntrinsicStmtNode::AssertEq {
                left,
                right,
                message,
                location: _location,
            } => {
                let left = self.visit_expr(left, ctx)?;
                let right = self.visit_expr(right, ctx)?;
                format!(
                    "assert_eq({}, {}, \"{}\")",
                    left,
                    right,
                    message.unwrap_or_default()
                )
            }
        };
        Ok(s)
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
    ) -> Result<Self::DefinitionResult, Self::Error> {
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

    fn visit_for(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let &ForNode {
            variable,
            start,
            end,
            body,
            location: ref _location,
        } = ctx.statement(node).as_for().unwrap();
        let s = format!(
            "for {} in {}..{} ",
            ctx.ident(variable).to_string(),
            self.visit_expr(start, ctx)?,
            self.visit_expr(end, ctx)?
        );
        let block = self.visit_block_expr(body, ctx)?;
        self.write_line(&format!("{}{}", s, block));
        Ok(Default::default())
    }

    fn visit_match(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let MatchNode {
            scrutinee,
            arms,
            location: _location,
        } = ctx.expression(node).as_match().cloned().unwrap();

        let mut m = format!("match {} {{ ", self.visit_expr(scrutinee.clone(), ctx)?);
        self.indent();
        let current_indent = self.read_indent(0);
        for arm in arms {
            let s = format!(
                " \n{}{} => ",
                current_indent,
                if arm.pattern.is_place_holder() {
                    "_".to_string()
                } else {
                    let (pattern_expr, _) = arm.pattern.as_value().unwrap();
                    self.visit_expr(pattern_expr.clone(), ctx)?
                }
            );
            let expr = self.visit_expr(arm.body, ctx)?;
            m.push_str(&format!("{}{},", s, expr));
        }
        self.dedent();
        m.push_str(&format!("\n{}}}", self.read_indent(0)));
        Ok(m)
    }

    fn visit_lambda_function(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let LambdaFunctionNode {
            parameters,
            return_type,
            ..
        } = ctx.expression(node).as_lambda_function().cloned().unwrap();
        let parameters = parameters
            .iter()
            .map(|p| {
                format!(
                    "{}{}: {}",
                    if p.qualifier.is_mutable { "mut " } else { "" },
                    &ctx.ident(p.name),
                    self.visit_unchecked_type(&p.ty, ctx)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let result = format!(
            "|{}|{} {}",
            parameters,
            if let Some(ref ret) = return_type {
                format!(" -> {}", self.visit_unchecked_type(&ret, ctx))
            } else {
                "".to_string()
            },
            "{ .. }".to_string()
        );

        Ok(result)
    }

    fn visit_trait_impl(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let TraitImplNode {
            generic_parameters,
            trait_ty,
            ty,
            body,
            location: _location,
        } = ctx.definition(def_id).as_trait_impl().unwrap();
        let generic_parameters = generic_parameters
            .iter()
            .map(|generic_parameter| ctx.ident(generic_parameter.name).to_string())
            .collect::<Vec<_>>();
        let generic_parameters = self.visit_generic_parameters(generic_parameters);

        let s = format!(
            "impl{} {}{} {{",
            generic_parameters,
            format!("{} for ", self.visit_unchecked_type(&trait_ty, ctx)),
            self.visit_unchecked_type(&ty, ctx),
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

    fn visit_block_expr(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let BlockExprNode {
            stmts,
            expr: return_expr,
            location: _location,
        } = ctx.expression(node).as_block_expr().unwrap().clone();

        let mut block_expr_result = String::new();
        block_expr_result.push_str(format!("{{\n").as_str());

        self.indent();
        let current_indent = self.read_indent(0);

        for stmt in stmts.iter() {
            let stmt_result = self.visit_stmt(*stmt, ctx)?;
            if !stmt_result.is_empty() {
                block_expr_result.push_str(&format!("{}{}\n", current_indent, stmt_result));
            }
        }
        //add return expr
        match return_expr {
            Some(expr) => {
                let return_expr_result = self.visit_expr(expr, ctx)?;
                block_expr_result.push_str(&format!("{}{}\n", current_indent, return_expr_result));
            }
            None => {}
        }

        self.dedent();
        block_expr_result.push_str(&self.read_indent(0));
        block_expr_result.push_str("}");

        Ok(block_expr_result)
    }

    fn visit_if_expr(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let IfExprNode {
            if_branch,
            elseif_branches,
            else_branch,
            ..
        } = ctx.expression(node).as_if_expr().unwrap().clone();

        let current_indent = self.read_indent(0);
        let mut result = format!("if {} ", self.visit_expr(if_branch.predicate, ctx)?);
        result.push_str(&format!("{}", self.visit_expr(if_branch.body, ctx)?));
        result.push_str(&format!("{}\n", current_indent));

        for branch in elseif_branches.into_iter() {
            result.push_str(&format!(
                "{}else if {} ",
                current_indent,
                self.visit_expr(branch.predicate, ctx)?
            ));
            result.push_str(&format!("{}", self.visit_expr(branch.body, ctx)?));
            result.push_str(&format!("{}\n", current_indent));
        }

        if let Some(else_branch) = else_branch {
            result.push_str(&format!("{}else ", current_indent));
            result.push_str(&format!("{}", self.visit_expr(else_branch, ctx)?));
        }
        Ok(result)
    }

    fn visit_tuple(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let tuple_node = ctx.expression(node).as_tuple().unwrap();
        let elements = tuple_node.elements.clone();

        let formatted_elements = elements
            .iter()
            .map(|&expr_id| self.visit_expr(expr_id, ctx))
            .collect::<Result<Vec<_>, Self::Error>>()?
            .join(", ");

        Ok(format!("({})", formatted_elements))
    }

    fn visit_tuple_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        // get tuple access node, avoid borrowing ctx twice in closure
        let tuple_access_node = ctx.expression(node).as_tuple_access().unwrap();
        let target_expr_id = tuple_access_node.target;

        let index = tuple_access_node.index;

        let tuple_expr = self.visit_expr(target_expr_id, ctx)?;

        Ok(format!("{}.{}", tuple_expr, index))
    }
}
