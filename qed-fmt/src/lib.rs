use qed_ast::*;
use qed_builder::{Context, ContextFelt};
use qed_parser::Parser;
use std::fmt::{Display, Write};

#[derive(Debug)]
pub struct Formatter<'a, F: Clone, C> {
    output: String,
    indent: usize,
    parser: &'a Parser<F, C>,
}

impl<'a, F: Clone + Display, C> Formatter<'a, F, C> {
    pub fn new(parser: &'a Parser<F, C>) -> Self {
        Formatter {
            output: String::new(),
            indent: 0,
            parser,
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

    fn visit_unchecked_type(&self, node: &UncheckedType) -> String {
        match node {
            UncheckedType::Basic(name) => self.parser.interner[name.clone()].to_string(),
            UncheckedType::Generic(name, generic_parameters) => format!(
                "{}{}",
                &self.parser.interner[name.clone()],
                self.visit_generic_parameters(
                    generic_parameters
                        .into_iter()
                        .map(|x| self.visit_unchecked_type(x))
                        .collect::<Vec<_>>()
                )
            ),
            UncheckedType::Array(ty, size) => {
                format!("[{};{}]", self.visit_unchecked_type(ty), size)
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

impl<'a, F: ContextFelt + Display, C: Context<F>> AstVisitor<F, C> for Formatter<'a, F, C> {
    type ExprResult = String;
    type StmtResult = String;
    type Context = ();
    type Error = ();

    fn visit_use(&mut self, u: &UsePath, ctx: &mut Self::Context) -> Result<(), Self::Error> {
        let mut path = vec![self.parser.interner[u.kind.clone().into()].to_string()];
        let mut segments = u
            .segments
            .iter()
            .map(|&s| self.parser.interner[s].to_string())
            .collect::<Vec<_>>();
        path.extend(segments);
        let target = u
            .target
            .map(|t| self.parser.interner[t].to_string())
            .unwrap_or("*".to_string());

        self.write_line(&format!("use {}::{};", path.join("::"), target));
        Ok(Default::default())
    }

    fn visit_path(
        &mut self,
        node: &PathNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(self.parser.interner[node.0].to_string())
    }

    fn visit_index_access(
        &mut self,
        node: &IndexAccessNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(format!(
            "{}[{}]",
            self.visit_expr(&self.parser.exprs[node.value], ctx)?,
            node.index
        ))
    }

    fn visit_member_access(
        &mut self,
        node: &MemberAccessNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(format!(
            "{}.{}",
            self.visit_expr(&self.parser.exprs[node.value], ctx)?,
            self.parser.interner[node.field]
        ))
    }

    fn visit_value(
        &mut self,
        node: &ValueNode<F>,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(match node {
            ValueNode::Felt(f) => f.to_string(),
            ValueNode::Bool(b) => b.to_string(),
            ValueNode::Array(_, values) => format!(
                "[{}]",
                values
                    .into_iter()
                    .map(|&v| self.visit_expr(&self.parser.exprs[v], ctx))
                    .collect::<Result<Vec<_>, Self::Error>>()?
                    .join(", ")
            ),
            ValueNode::Struct(name, generic_parameters, field_values) => {
                let name = &self.parser.interner[name.clone()];
                let generic_parameters = self.visit_generic_parameters(
                    generic_parameters
                        .iter()
                        .map(|p| self.visit_unchecked_type(p))
                        .collect::<Vec<_>>(),
                );
                let mut result = format!("{}{} {{\n", name, generic_parameters);

                for (&field, value) in field_values {
                    result.push_str(&self.read_indent(1));
                    result.push_str(&format!(
                        "{}: {},\n",
                        self.parser.interner[field],
                        self.visit_expr(&self.parser.exprs[*value], ctx)?
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
        node: &BinaryNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(format!(
            "{} {} {}",
            self.visit_expr(&self.parser.exprs[node.lhs], ctx)?,
            node.operator,
            self.visit_expr(&self.parser.exprs[node.rhs], ctx)?
        ))
    }

    fn visit_unary(
        &mut self,
        node: &UnaryNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(format!(
            "{}{}",
            node.operator,
            self.visit_expr(&self.parser.exprs[node.rhs], ctx)?
        ))
    }

    fn visit_call(
        &mut self,
        node: &CallNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        let args = node
            .args
            .iter()
            .map(|&arg| self.visit_expr(&self.parser.exprs[arg], ctx))
            .collect::<Result<Vec<_>, Self::Error>>()?
            .join(", ");
        Ok(format!(
            "{}({})",
            self.visit_expr(&self.parser.exprs[node.variable], ctx)?,
            args
        ))
    }

    fn visit_cast(
        &mut self,
        node: &CastNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        Ok(format!(
            "({} as {})",
            self.visit_expr(&self.parser.exprs[node.value], ctx)?,
            self.visit_unchecked_type(&node.target_type)
        ))
    }

    fn visit_if(
        &mut self,
        node: &IfNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let mut result = format!(
            "if {} {{",
            self.visit_expr(&self.parser.exprs[node.if_branch.predicate], ctx)?
        );
        self.write_line(&result);
        self.indent();
        self.visit_block(&node.if_branch.body, ctx);
        self.dedent();
        self.write("}");

        for branch in &node.elseif_branch {
            let s = format!(
                " else if {} {{",
                self.visit_expr(&self.parser.exprs[branch.predicate], ctx)?
            );
            self.append_line(&s);
            self.indent();
            self.visit_block(&branch.body, ctx);
            self.dedent();
            self.write("}");
        }

        if let Some(else_branch) = &node.else_branch {
            self.append_line(" else {");
            self.indent();
            self.visit_block(else_branch, ctx);
            self.dedent();
            self.write("}");
        }

        self.append_line(";");
        Ok(Default::default())
    }

    fn visit_while(
        &mut self,
        node: &WhileNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let mut s = format!(
            "while {} {{",
            self.visit_expr(&self.parser.exprs[node.predicate], ctx)?
        );
        self.write_line(&s);
        self.indent();
        self.visit_block(&node.body, ctx);
        self.dedent();
        self.write_line("};");
        Ok(Default::default())
    }

    fn visit_block(
        &mut self,
        node: &BlockNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        for stmt in &node.stmts {
            self.visit_stmt(&self.parser.stmts[stmt.clone()], ctx)?;
        }
        Ok(Default::default())
    }

    fn visit_assignment(
        &mut self,
        node: &AssignmentNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let s = format!(
            "{} = {};",
            self.visit_expr(&self.parser.exprs[node.variable], ctx)?,
            self.visit_expr(&self.parser.exprs[node.value], ctx)?
        );
        self.write_line(&s);
        Ok(Default::default())
    }

    fn visit_variable(
        &mut self,
        node: &VariableNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let s = format!(
            "{}{} {}: {} = {};",
            if node.cnst { "const" } else { "let" },
            if node.mutable { " mut" } else { "" },
            &self.parser.interner[node.name],
            self.visit_unchecked_type(&node.ty),
            self.visit_expr(&self.parser.exprs[node.value], ctx)?
        );
        self.write_line(&s);
        Ok(Default::default())
    }

    fn visit_return(
        &mut self,
        node: &ReturnNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let s = format!(
            "return{};",
            if let Some(ret) = node.0 {
                format!(" {}", self.visit_expr(&self.parser.exprs[ret], ctx)?)
            } else {
                "".to_string()
            }
        );
        self.write_line(&s);
        Ok(Default::default())
    }

    fn visit_impl(
        &mut self,
        node: &ImplNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let generic_parameters = node
            .generic_parameters
            .iter()
            .map(|&generic_parameter| self.parser.interner[generic_parameter].to_string())
            .collect::<Vec<_>>();
        let generic_parameters = self.visit_generic_parameters(generic_parameters);

        let mut s = format!(
            "impl{} {}{} {{",
            generic_parameters, &self.parser.interner[node.ty], generic_parameters
        );
        self.write_line(&s);
        self.indent();
        for func in &node.body {
            self.visit_function(func, ctx)?;
        }
        self.dedent();
        self.write_line("}");
        Ok(Default::default())
    }

    fn visit_function(
        &mut self,
        node: &FunctionNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        let parameters = node
            .parameters
            .iter()
            .map(|p| {
                format!(
                    "{}{}: {}",
                    if p.1 { "mut " } else { "" },
                    &self.parser.interner[p.0],
                    self.visit_unchecked_type(&p.2)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let generic_parameters = node
            .generic_parameters
            .iter()
            .map(|x| self.parser.interner[x.clone()].to_string())
            .collect::<Vec<_>>();
        let mut s = format!(
            "fn {}{}({}){} {{",
            self.parser.interner[node.name.clone()],
            self.visit_generic_parameters(generic_parameters),
            parameters,
            if let Some(ref ret) = node.return_type {
                format!(" -> {}", self.visit_unchecked_type(&ret))
            } else {
                "".to_string()
            }
        );
        self.write_line(&s);
        self.indent();
        self.visit_block(node.body.as_ref().unwrap(), ctx);
        self.dedent();
        self.write_line("}");
        Ok(Default::default())
    }

    fn visit_struct(
        &mut self,
        node: &StructNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        for attr in &node.attrs {
            if !attr.properties.is_empty() {
                self.write_line(&format!(
                    "#[{}({})]",
                    self.parser.interner[attr.name],
                    attr.properties
                        .iter()
                        .map(|p| self.parser.interner[p.clone()].to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            } else {
                self.write_line(&format!("#[{}]", self.parser.interner[attr.name],));
            }
        }
        self.write_line(&format!(
            "struct {}{} {{",
            &self.parser.interner[node.name],
            self.visit_generic_parameters(
                node.generic_parameters
                    .iter()
                    .map(|p| self.parser.interner[p.clone()].to_string())
                    .collect::<Vec<_>>()
            )
        ));
        self.indent();
        for (field, value) in &node.fields {
            let s = format!(
                "{}: {},",
                self.parser.interner[field.clone()],
                self.visit_unchecked_type(&value)
            );
            self.write_line(&s);
        }
        self.dedent();
        self.write_line("}");
        Ok(Default::default())
    }

    fn visit_enum(
        &mut self,
        node: &EnumNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        self.write_line(&format!(
            "enum {}{} {{",
            &self.parser.interner[node.name],
            self.visit_generic_parameters(
                node.generic_parameters
                    .iter()
                    .map(|p| self.parser.interner[p.clone()].to_string())
                    .collect::<Vec<_>>()
            )
        ));
        self.indent();
        for variant in &node.variants {
            match variant {
                EnumVariant::Basic(ident_id) => {
                    let s = format!("{}", self.parser.interner[ident_id.clone()]);
                    self.write_line(&s);
                }
                EnumVariant::Tuple(ident_id, types) => {
                    let types = types
                        .iter()
                        .map(|x| self.visit_unchecked_type(x))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let s = format!("{}({})", self.parser.interner[ident_id.clone()], types);
                    self.write_line(&s);
                }
                EnumVariant::Struct(ident_id, fields) => {
                    self.write_line(&format!("{} {{", self.parser.interner[ident_id.clone()]));
                    self.indent();
                    for (field, ty) in fields {
                        self.write_line(&format!(
                            "{}: {},",
                            self.parser.interner[field.clone()],
                            self.visit_unchecked_type(ty)
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
        node: &TraitNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        self.write_line(&format!(
            "trait {}{} {{",
            &self.parser.interner[node.name],
            self.visit_generic_parameters(
                node.generic_parameters
                    .iter()
                    .map(|p| self.parser.interner[p.clone()].to_string())
                    .collect::<Vec<_>>()
            )
        ));
        self.indent();
        for func in &node.body {
            let parameters = func
                .parameters
                .iter()
                .map(|p| {
                    format!(
                        "{}{}: {}",
                        if p.1 { "mut " } else { "" },
                        &self.parser.interner[p.0],
                        self.visit_unchecked_type(&p.2)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let s = format!(
                "fn {}({}){};",
                self.parser.interner[func.name.clone()],
                parameters,
                if let Some(ref ret) = func.return_type {
                    format!(" -> {}", self.visit_unchecked_type(&ret))
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
