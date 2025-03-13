use indexmap::IndexMap;
use qed_ast::{
    AssignmentNode, BinaryNode, BlockExprNode, CallNode, CastNode, DefId, DefinitionNode, EnumNode,
    EnumVariant, ExprId, ExprNode, ForNode, FunctionNode, Ident, IdentId, IfExprNode, ImplNode,
    ImplTraitNode, IndexAccessNode, IntrinsicExprNode, IntrinsicStmtNode, LambdaFunctionNode,
    MatchNode, MemberAccessNode, MemberCallNode, ModuleId, NodeId, NodeInfo, NodeType, ReturnNode,
    Span, StmtId, StmtNode, StructNode, TraitNode, TypeQualifier, UnaryNode, UncheckedType,
    ValueNode, Visibility, VisitorContext, WhileNode,
};
use qedlang_core::dpn::ops::context_trait::ContextFelt;
use std::fmt::{format, Display};
use std::ops::Deref;
// debug_ident
// debug_scope
// debug_module
// debug_trait
// debug_path
// debug_expr
// debug_stmt
// debug_def

use crate::{
    CheckedDefinitionNode, CheckedExprNode, CheckedStmtNode, Error, Scope, ScopeId, Type,
    TypeChecker, TypeCheckerVisitorContext, TypeId, TypeKey,
};

pub trait AstVisualizer<F: Clone + From<u32>, C>: VisitorContext<F, C> {
    type DebugResult;

    fn debug_scope(&self, scope_id: ScopeId) -> Self::DebugResult;
    fn debug_function(&self, def_id: DefId) -> Self::DebugResult;
    fn debug_struct(&self, def_id: DefId) -> Self::DebugResult;
    fn debug_type(&self, type_id: TypeId) -> Self::DebugResult;
    fn debug_type_name(&self, type_id: TypeId) -> Self::DebugResult;
}

pub struct IndentFormatter {
    output: String,
    indent: usize,
}

impl IndentFormatter {
    const INDENT: &'static str = "    ";

    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    pub fn indent(&mut self) {
        self.indent += 1;
    }

    pub fn dedent(&mut self) {
        self.indent -= 1;
    }

    pub fn write<T: AsRef<str>>(&mut self, s: T) {
        self.output.push_str(s.as_ref());
    }

    pub fn writeln<T: AsRef<str>>(&mut self, s: T) {
        self.write_indent();
        self.output.push_str(s.as_ref());
        self.output.push_str("\n");
    }

    pub fn write_indent(&mut self) {
        self.output.push_str(&Self::INDENT.repeat(self.indent));
    }

    pub fn finish(self) -> String {
        self.output
    }
}

pub struct TypeCheckerVisitorVisualizerInner<'a, F: Clone + From<u32> + ContextFelt, C> {
    pub context: &'a TypeCheckerVisitorContext<F, C>,
}

impl<'a, F: Clone + From<u32> + ContextFelt, C> TypeCheckerVisitorVisualizerInner<'a, F, C> {
    pub fn debug_scope(&self, scope_id: ScopeId, formatter: &mut IndentFormatter) {
        let scope = &self.context.symbols[scope_id];
        formatter.writeln(format!("Kind: {:?}", scope.kind));
        formatter.writeln("Constants:");
        formatter.indent();
        for (indent_id, const_id) in &scope.consts {
            formatter.writeln(format!(
                "{:?}: {:?}",
                const_id,
                self.context.ident(*indent_id)
            ));
        }
        formatter.dedent();

        formatter.writeln("Variables:");
        formatter.indent();
        for (indent_id, var_id) in &scope.variables {
            formatter.writeln(format!(
                "{:?}: {:?}",
                var_id,
                self.context.ident(*indent_id)
            ));
        }
        formatter.dedent();

        formatter.writeln("Types:");
        formatter.indent();
        for (type_key, type_id) in &scope.types {
            self.debug_type(type_key, type_id, formatter);
        }
        formatter.dedent();

        formatter.writeln("")
    }

    pub fn debug_type(&self, type_key: &TypeKey, type_id: &TypeId, fmt: &mut IndentFormatter) {
        let ty = &self.context.symbols[*type_id];
        let ident_id = ty.name();
        let ident_name = &self.context.ident(ident_id).0;
        let type_kind = ty.kind();

        fmt.writeln(format!("{}: ", ident_name));

        fmt.indent();
        fmt.writeln(format!("IndentId: {:?}", ident_id));
        fmt.writeln(format!("TypeId: {:?}", type_id));
        fmt.writeln(format!("Type: {:?}", type_kind));
        fmt.writeln(format!("Visibility: {:?}", ty.visibility()));

        match &ty {
            Type::Unknown => {}
            Type::VOID => {}
            Type::Felt(_) => {}
            Type::Bool(_) => {}
            Type::U32(_) => {}
            Type::Array(_) => {}
            Type::Struct(_) => {}
            Type::Enum(_) => {}
            Type::Function(_) => {}
            Type::Trait(_) => {}
            Type::Const(_) => {}
            Type::LambdaFunction(_) => {}
            Type::FunctionSignature(_) => {}
            Type::TypeVariable(_) => {}
            Type::Tuple(_) => {}
            Type::GenericInstance(_, _, _) => {}
        }
        if !type_key.generic_parameters.is_empty() {
            fmt.writeln(format!(
                "Generic Parameters: {:?}",
                type_key.generic_parameters
            ));
        }
        if let Some(return_type) = type_key.return_type {
            fmt.writeln(format!(
                "Return Type: {:?}",
                self.context.debug_type(return_type)
            ));
        }
        if !type_key.consts.is_empty() {
            fmt.writeln(format!("Consts: {:?}", type_key.consts));
        }
        if let Some(underlying_type_id) = type_key.underlying_type_id {
            fmt.writeln(format!(
                "Underlying Type: {:?}",
                self.context.debug_type(underlying_type_id)
            ));
        }
        if !type_key.parameters.is_empty() {
            fmt.writeln(format!("Parameters: {:?}", type_key.parameters));
        }
        // fmt.writeln(format!(
        //     "Type Content: {}",
        //     self.context.debug_type(*type_id)
        // ));
        self.debug_definition(*type_id, fmt);
        fmt.dedent();
    }

    pub fn debug_definition(&self, type_id: TypeId, fmt: &mut IndentFormatter) {
        match &self.context.symbols[type_id] {
            Type::Struct(node) => {
                // fmt.writeln(format!("Fields: {:?}", node.fields));
                if node.fields.len() == 0 {
                    fmt.writeln("Fields: []");
                } else {
                    fmt.writeln("Fields:");
                    fmt.indent();
                    for (ident_id, (type_id, visibility)) in node.fields.iter() {
                        let ty = &self.context.symbols[*type_id];
                        let ident_name = &self.context.ident(*ident_id).0;
                        fmt.writeln(format!("{}:", ident_name));
                        fmt.indent();
                        // abc (Const) Private, Const(10)
                        // IdentId, TypeId (x)
                        //
                        fmt.writeln(format!("Type: {:?}", ty.kind()));
                        fmt.writeln(format!("IndentId: {:?}", ident_id));
                        fmt.writeln(format!("TypeId: {:?}", type_id));
                        fmt.writeln(format!("Visibility: {:?}", visibility));
                        fmt.dedent();
                    }
                    fmt.dedent();
                };
                fmt.writeln(format!("Generic Parameters: {:?}", node.generic_parameters));
                fmt.writeln(format!("Implementation: {:?}", node.implementations));
                fmt.writeln(format!("ScopeId: {:?}", node.scope_id));
                fmt.writeln("Span:");
                fmt.indent();
                fmt.writeln(format!("{:?}", node.span.file_id));
                fmt.writeln(format!("Start: {}", node.span.start));
                fmt.writeln(format!("End: {}", node.span.end));
                fmt.dedent();
            }
            Type::Function(node) => {
                if node.parameters.len() == 0 {
                    fmt.writeln("Parameters: []");
                } else {
                    fmt.writeln("Parameters:");
                    fmt.indent();
                    for (ident_id, type_qualifier, type_id) in node.parameters.iter() {
                        let ty = &self.context.symbols[*type_id];
                        let ident_name = &self.context.ident(*ident_id).0;
                        fmt.writeln(format!("{}:", ident_name));
                        fmt.indent();
                        fmt.writeln(format!("Type: {:?}", ty.kind()));
                        fmt.writeln(format!("IndentId: {:?}", ident_id));
                        fmt.writeln(format!("TypeId: {:?}", type_id));
                        fmt.writeln(format!("Mutable: {}", type_qualifier.is_mutable));
                        fmt.dedent();
                    }
                    fmt.dedent();
                };
                fmt.writeln(format!("Body: {:?}", node.body));
                fmt.writeln(format!("Return Type: {:?}", node.return_type));
                fmt.writeln(format!("Generic Parameters: {:?}", node.generic_parameters));
                fmt.writeln(format!("Attrs: {:?}", node.attrs));
                fmt.writeln(format!("Is Extern: {}", node.qualifier.is_extern));
                fmt.writeln(format!("Is Const: {}", node.qualifier.is_const));
                fmt.writeln(format!("ScopeId: {:?}", node.scope_id));
                fmt.writeln("Span:");
                fmt.indent();
                fmt.writeln(format!("{:?}", node.span.file_id));
                fmt.writeln(format!("Start: {}", node.span.start));
                fmt.writeln(format!("End: {}", node.span.end));
                fmt.dedent();
            }
            Type::Const(checked_const_node) => match checked_const_node.name {
                Some(name) => {
                    let line = format!(
                        "Definition: {:?}",
                        self.context.symbols.get_constant(checked_const_node.value)
                    );
                    fmt.writeln(line);
                }
                None => {
                    fmt.writeln(format!("Definition: {:?}", checked_const_node));
                }
            },
            Type::Array(node) => {
                let type_name = self.get_type_name(node.inner_ty);
                let size_name = self.get_type_name(node.size_ty);
                // let type_name = &self.context.ident(node.inner_ty).0;
                // let size_name = &self.context.ident(node.size_ty).0;
                fmt.writeln(format!(
                    "Array [{}; {}], {:?}",
                    type_name, size_name, node.implementations
                ));
            }
            x => fmt.writeln(format!("Remaining {:?}", x)),
        }
    }

    pub fn get_type_name(&self, type_id: TypeId) -> String {
        match &self.context.symbols[type_id] {
            Type::Unknown => "Unknown".to_string(),
            Type::VOID => "Void".to_string(),
            Type::Felt(_) => "Felt".to_string(),
            Type::Bool(_) => "Bool".to_string(),
            Type::U32(_) => "U32".to_string(),
            Type::Array(_) => "Array".to_string(),
            Type::Struct(node) => self.context.ident(node.name).0.to_string(),
            // Type::TypeVariable(node) => {
            //     let len = node.constraints.len();
            //     match len {
            //         // 0 => self.context.debug_type_name(type_id),
            //         // 0 => self.context.
            //         0 => unreachable!(),
            //         _ => {
            //             let mut type_variable_details = vec![];
            //             for type_id in node.constraints.iter() {
            //                 type_variable_details.push(self.debug_type(type_id.clone()));
            //             }
            //             format!("{}", type_variable_details.join(" + "))
            //         }
            //     }
            // }
            // Type::Enum(node) => self.context.ident(node.name).into(),
            // Type::Function(node) => self.context.ident(node.name).into(),
            node => {
                format!("?Type: {:?}", node)
            }
        }
    }
}

impl<F: Clone + From<u32> + ContextFelt, C> AstVisualizer<F, C>
    for TypeCheckerVisitorContext<F, C>
{
    type DebugResult = String;
    fn debug_scope(&self, scope_id: ScopeId) -> Self::DebugResult {
        let scope = &self.symbols[scope_id];

        let variables = scope
            .variables
            .iter()
            .map(|(ident_id, var_id)| format!("\n{:?}: {:?}", var_id, self.ident(*ident_id)))
            .collect::<Vec<_>>()
            .join(", ");
        let consts = scope
            .consts
            .iter()
            .map(|(ident_id, const_id)| format!("\n{:?}: {:?}", const_id, self.ident(*ident_id)))
            .collect::<Vec<_>>()
            .join(", ");
        let types = scope
            .types
            .iter()
            .map(
                |(type_key, type_id)| format!("\n{:?}: {:?}", type_key, self.debug_type(*type_id),),
            )
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"ScopeId({}) {{
    variables: {}
    consts: {}
    types: {}
}}"#,
            usize::from(scope_id),
            variables,
            consts,
            types
        )
    }

    fn debug_function(&self, def_id: DefId) -> Self::DebugResult {
        todo!()
    }

    fn debug_struct(&self, def_id: DefId) -> Self::DebugResult {
        todo!()
    }

    fn debug_type(&self, type_id: TypeId) -> Self::DebugResult {
        match &self.symbols[type_id] {
            Type::Unknown => format!("Unknown"),
            Type::VOID => format!("void"),
            Type::Felt(checked_felt_node) => format!("Felt"),
            Type::Bool(checked_bool_node) => format!("Bool"),
            Type::U32(checked_u32_node) => format!("U32"),
            Type::Array(checked_array_node) => {
                format!(
                    "Array [{}; {}]",
                    self.debug_type_name(checked_array_node.inner_ty),
                    self.debug_type_name(checked_array_node.size_ty),
                )
            }
            Type::Struct(checked_struct_node) => {
                let struct_fields = checked_struct_node
                    .fields
                    .iter()
                    .map(|(filed_id, (field_type_id, visibility))| {
                        format!(
                            "{}{}: {}",
                            if visibility == &Visibility::Public {
                                "pub "
                            } else {
                                ""
                            },
                            self.ident(*filed_id),
                            self.debug_type(*field_type_id)
                        )
                    })
                    .collect::<Vec<_>>();
                format!(
                    "{}struct {} {{{}}}",
                    if checked_struct_node.visibility == Visibility::Public {
                        "pub "
                    } else {
                        ""
                    },
                    self.ident(checked_struct_node.name),
                    struct_fields.join(",\n ")
                )
            }
            Type::Enum(checked_enum_node) => format!("Enum {}", self.ident(checked_enum_node.name)),
            Type::Function(checked_function_node) => {
                format!(
                    "fn {}. {:?}",
                    self.ident(checked_function_node.name),
                    checked_function_node
                )
            }
            Type::Trait(checked_trait_node) => {
                format!("Trait {}", self.ident(checked_trait_node.name))
            }
            Type::Const(checked_const_node) => match checked_const_node.name {
                Some(name) => {
                    format!(
                        "{}const {} {:?}",
                        if checked_const_node.visibility == Visibility::Public {
                            "pub "
                        } else {
                            ""
                        },
                        self.ident(name),
                        self.symbols.get_constant(checked_const_node.value)
                    )
                }
                None => self.debug_type_name(type_id),
            },
            Type::LambdaFunction(checked_lambda_function_node) => {
                format!("lamba fn {}", self.ident(checked_lambda_function_node.name))
            }
            Type::FunctionSignature(checked_function_signature) => format!("fn sig"),
            Type::TypeVariable(checked_type_variable_node) => {
                let len = checked_type_variable_node.constraints.len();
                match len {
                    0 => self.debug_type_name(type_id),
                    _ => {
                        let mut type_variable_details = vec![];
                        for type_id in checked_type_variable_node.constraints.iter() {
                            type_variable_details.push(self.debug_type(type_id.clone()));
                        }
                        format!("{}", type_variable_details.join(" + "))
                    }
                }
            }
            Type::Tuple(type_ids) => {
                let tuple_elems = type_ids
                    .iter()
                    .map(|type_id| self.debug_type(*type_id))
                    .collect::<Vec<_>>();

                format!("({})", tuple_elems.join(", "))
            }
            Type::GenericInstance(type_id, type_ids, scope_id) => match &self.symbols[*type_id] {
                Type::Array(_) => {
                    format!(
                        "[{}; {}]",
                        self.debug_type_name(type_ids[0]),
                        self.debug_type_name(type_ids[1])
                    )
                }
                Type::Struct(_) => {
                    let struct_name = self.debug_type_name(*type_id);
                    let generic_args = type_ids
                        .iter()
                        .map(|type_id| self.debug_type(*type_id))
                        .collect::<Vec<_>>();
                    format!("{}<{}>", struct_name, generic_args.join(", "))
                }
                _ => {
                    unimplemented!()
                }
            },
        }
    }

    fn debug_type_name(&self, type_id: TypeId) -> Self::DebugResult {
        match &self.symbols[type_id] {
            Type::Unknown => format!("Unknown"),
            Type::VOID => format!("void"),
            Type::Felt(_checked_felt_node) => format!("Felt"),
            Type::Bool(_checked_bool_node) => format!("Bool"),
            Type::U32(_checked_u32_node) => format!("u32"),
            Type::Array(checked_array_node) => {
                format!(
                    "[{}; {}]",
                    self.debug_type_name(checked_array_node.inner_ty),
                    self.debug_type_name(checked_array_node.size_ty),
                )
            }
            Type::Struct(checked_struct_node) => {
                format!("{}", self.ident(checked_struct_node.name))
            }
            Type::Enum(checked_enum_node) => format!("{}", self.ident(checked_enum_node.name)),
            Type::Trait(checked_trait_node) => {
                format!("{}", self.ident(checked_trait_node.name))
            }
            Type::Const(checked_const_node) => {
                let name = checked_const_node.name;
                match name {
                    Some(name) => {
                        format!("{}", self.ident(name))
                    }
                    None => {
                        format!("{:?}", self.symbols.get_constant(checked_const_node.value))
                    }
                }
            }
            Type::Tuple(type_ids) => {
                let tuple_elems = type_ids
                    .iter()
                    .map(|type_id| self.debug_type_name(*type_id))
                    .collect::<Vec<_>>();

                format!("({})", tuple_elems.join(", "))
            }
            Type::Function(node) => {
                format!("fn {:?} {:?}", self.ident(node.name), node)
            }
            Type::LambdaFunction(node) => {
                format!("lambda fn {:?}", self.ident(node.name))
            }
            Type::FunctionSignature(node) => {
                format!("fn sig {:?}", node)
            }
            Type::TypeVariable(node) => {
                format!("type variable {:?}", node)
            }
            Type::GenericInstance(type_id, type_ids, scope_id) => {
                format!("generic instance {:?}", type_id)
                // format!(
                //     "{}<{}>",
                //     self.debug_type_name(*type_id),
                //     type_ids.iter().map(String::from).join(", ")
                // )
            }
        }
    }
}
