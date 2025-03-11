use qed_ast::{
    AssignmentNode, BinaryNode, BlockExprNode, CallNode, CastNode, DefId, DefinitionNode, EnumNode,
    EnumVariant, ExprId, ExprNode, ForNode, FunctionNode, IdentId, IfExprNode, ImplNode,
    ImplTraitNode, IndexAccessNode, IntrinsicExprNode, IntrinsicStmtNode, LambdaFunctionNode,
    MatchNode, MemberAccessNode, MemberCallNode, ModuleId, NodeId, NodeInfo, NodeType, ReturnNode,
    StmtId, StmtNode, StructNode, TraitNode, UnaryNode, UncheckedType, ValueNode, Visibility,
    VisitorContext, WhileNode,
};
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{
    CheckedDefinitionNode, CheckedExprNode, CheckedStmtNode, Error, ScopeId, Type, TypeChecker,
    TypeCheckerVisitorContext, TypeId,
};

pub trait AstVisualizer<F: Clone + From<u32>, C>: VisitorContext<F, C> {
    type DebugResult;

    fn debug_scope(&self, scope_id: ScopeId) -> Self::DebugResult;
    fn debug_function(&self, def_id: DefId) -> Self::DebugResult;
    fn debug_struct(&self, def_id: DefId) -> Self::DebugResult;
    fn debug_type(&self, type_id: TypeId) -> Self::DebugResult;
    fn debug_type_name(&self, type_id: TypeId) -> Self::DebugResult;
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
                format!("fn {}", self.ident(checked_function_node.name))
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
            _ => "Unknown Type".to_string(),
        }
    }
}
