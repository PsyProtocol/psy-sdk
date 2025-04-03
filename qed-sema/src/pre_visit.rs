use itertools::Itertools;
use qed_ast::*;

use indexmap::IndexMap;
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{
    CheckedDefinitionNode, CheckedFunctionNode, CheckedFunctionParameter, CheckedImplNode,
    CheckedStructNode, CheckedTraitNode, CheckedVariable, Error, Implementer, Result, ScopeKind,
    Type, TypeChecker, TypeCheckerVisitorContext, UNKOWN_TYPE, VOID_TYPE,
};

impl<F: Clone + From<u32> + ContextFelt, C> TypeChecker<F, C> {
    pub fn visit_module_definition_step1(
        &mut self,
        def_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let node = ctx.definition(def_id).clone();
        match node {
            DefinitionNode::Function(function_node) => {
                ctx.symbols.start_scope(ScopeKind::Function);
                self.infcx.enter_context();

                let checked_function = CheckedFunctionNode {
                    name: function_node.name,
                    parameters: vec![],
                    qualifier: function_node.qualifier,
                    generic_parameters: vec![],
                    body: None,
                    return_type: VOID_TYPE,
                    scope_id: ctx.symbols.current_scope_id().unwrap(),
                    visibility: function_node.visibility,
                    attrs: function_node.attrs,
                    type_id: UNKOWN_TYPE,
                    location: function_node.location,
                    comments: function_node.comments,
                };

                let ty = Type::Function(checked_function);
                let type_id = ctx
                    .symbols
                    .add_type(ctx.symbols.parent_scope_id(), ty.name(), ty)?;
                ctx.symbols[type_id].as_function_mut().unwrap().type_id = type_id;
                self.infcx.exit_context();
                ctx.symbols.end_scope();
            }
            DefinitionNode::Struct(struct_node) => {
                ctx.symbols.start_scope(ScopeKind::Struct);
                self.infcx.enter_context();

                let checked_struct = CheckedStructNode {
                    name: struct_node.name.clone(),
                    generic_parameters: vec![],
                    fields: IndexMap::new(),
                    scope_id: ctx.symbols.current_scope_id().unwrap(),
                    visibility: struct_node.visibility,
                    location: struct_node.location,
                    comments: struct_node.comments,
                };

                let ty = Type::Struct(checked_struct);
                ctx.symbols
                    .add_type(ctx.symbols.parent_scope_id(), struct_node.name.id, ty)
                    .unwrap();

                self.infcx.exit_context();
                ctx.symbols.end_scope();
            }
            DefinitionNode::Enum(_enum_node) => {}
            DefinitionNode::Impl(_impl_node) => {}
            DefinitionNode::TraitImpl(_impl_trait_node) => {}
            DefinitionNode::Trait(trait_node) => {
                ctx.symbols.start_scope(ScopeKind::Trait);
                self.infcx.enter_context();

                let checked_trait = CheckedTraitNode {
                    generic_parameters: vec![],
                    name: trait_node.name,
                    body: vec![],
                    unchecked_body: trait_node.body.clone(),
                    scope_id: ctx.symbols.current_scope_id().unwrap(),
                    visibility: trait_node.visibility,
                    location: trait_node.location,
                    comments: trait_node.comments,
                };

                let ty = Type::Trait(checked_trait);
                ctx.symbols
                    .add_type(ctx.symbols.parent_scope_id(), trait_node.name.id, ty)
                    .unwrap();

                self.infcx.exit_context();
                ctx.symbols.end_scope();
            }
            DefinitionNode::TypeAlias(_type_alias_node) => {}
            DefinitionNode::Const(_const_node) => {}
            DefinitionNode::Use(_use_node) => {}
        }

        Ok(())
    }

    pub fn visit_module_definition_step2(
        &mut self,
        def_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        ctx.push_node_id(NodeId::from(def_id));
        match ctx.definition(def_id).node_type() {
            NodeType::FunctionDef => {
                // visit function signature
                let function = ctx.definition(def_id).as_function().cloned().unwrap();
                let function_type_id = ctx.symbols.get_type_id(None, function.name.id).unwrap();
                let function_scope_id = ctx.symbols[function_type_id].scope_id();

                ctx.symbols.enter_scope(function_scope_id);
                self.infcx.enter_context();

                let checked_function = self.typecheck_function_signature(function, ctx)?;
                ctx.symbols[function_type_id] = Type::Function(checked_function.clone());

                self.infcx.exit_context();
                ctx.symbols.exit_scope();
            }
            NodeType::StructDef => {
                self.visit_struct(def_id, ctx)?;
            }
            NodeType::EnumDef => {}
            NodeType::ImplDef => {}
            NodeType::TraitImplDef => {}
            NodeType::TraitDef => {
                self.visit_trait(def_id, ctx)?;
            }
            NodeType::TypeAliasDef => {
                self.visit_type_alias(def_id, ctx)?;
            }
            NodeType::ConstDef => {}
            NodeType::UseDef => {
                self.visit_use(def_id, ctx)?;
            }
            _ => std::unreachable!(),
        };
        ctx.pop_node_id();
        Ok(())
    }

    pub fn visit_module_definition_step3(
        &mut self,
        def_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        ctx.push_node_id(NodeId::from(def_id));
        match ctx.definition(def_id).node_type() {
            NodeType::FunctionDef => {
                // visit function body
                let function = ctx.definition(def_id).as_function().cloned().unwrap();
                let function_type_id = ctx.symbols.get_type_id(None, function.name.id).unwrap();
                let function_scope_id = ctx.symbols[function_type_id].scope_id();

                ctx.symbols.enter_scope(function_scope_id);
                self.infcx.enter_context();

                let checked_function = self.typecheck_function_body(function, ctx)?;
                ctx.symbols[function_type_id] = Type::Function(checked_function.clone());

                self.infcx.exit_context();
                ctx.symbols.exit_scope();

                self.program
                    .defs
                    .alloc_item(CheckedDefinitionNode::Function(checked_function));
            }
            NodeType::StructDef => {}
            NodeType::EnumDef => {
                self.visit_enum(def_id, ctx)?;
            }
            NodeType::ImplDef => {
                self.visit_impl(def_id, ctx)?;
            }
            NodeType::TraitImplDef => {
                self.visit_trait_impl(def_id, ctx)?;
            }
            NodeType::TraitDef => {}
            NodeType::TypeAliasDef => {}
            NodeType::ConstDef => {
                self.visit_const(def_id, ctx)?;
            }
            NodeType::UseDef => {}
            _ => std::unreachable!(),
        };
        ctx.pop_node_id();
        Ok(())
    }

    fn typecheck_function_signature(
        &mut self,
        function: FunctionNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        let mut checked_generic_parameters = Vec::with_capacity(function.generic_parameters.len());
        for generic_parameter in &function.generic_parameters {
            let checked_generic_parameter =
                self.typecheck_generic_parameter(generic_parameter, ctx)?;
            let type_id = ctx
                .symbols
                .add_type_variable(ScopeKind::Impl, checked_generic_parameter)?;
            checked_generic_parameters.push(type_id);
        }

        let mut parameters = Vec::new();

        for parameter in &function.parameters {
            let parameter_type = self.typecheck(&parameter.ty, ctx)?;
            let variable = CheckedVariable::new(
                parameter.name,
                parameter_type,
                parameter.qualifier,
                ctx.symbols.current_scope_id().unwrap(),
                parameter.location,
            );
            ctx.symbols
                .declare_variable(variable)
                .ok_or(Error::VariableAlreadyDefined {
                    location: function.location,
                    variable: parameter.name.id,
                })?;
            parameters.push(CheckedFunctionParameter::new(
                parameter.name,
                parameter.qualifier,
                parameter_type,
                parameter.location,
            ));
        }

        let expected_return_type = if let Some(ref ret) = function.return_type {
            self.typecheck(ret, ctx)?
        } else {
            VOID_TYPE
        };

        match ctx.symbols.get_type_id(None, function.name.id) {
            Some(function_type_id) => {
                let mut checked_function = ctx.symbols[function_type_id]
                    .clone()
                    .into_function()
                    .unwrap();
                checked_function.generic_parameters = checked_generic_parameters;
                checked_function.parameters = parameters;
                checked_function.return_type = expected_return_type;
                Ok(checked_function)
            }
            None => Ok(CheckedFunctionNode {
                name: function.name,
                parameters: parameters,
                generic_parameters: checked_generic_parameters,
                body: None,
                qualifier: function.qualifier,
                return_type: expected_return_type,
                scope_id: ctx.symbols.current_scope_id().unwrap(),
                visibility: function.visibility,
                attrs: function.attrs,
                type_id: UNKOWN_TYPE,
                location: function.location,
                comments: function.comments,
            }),
        }
    }

    fn typecheck_function_body(
        &mut self,
        function: FunctionNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        let function_type_id = ctx.symbols.get_type_id(None, function.name.id).unwrap();

        let mut checked_function = ctx.symbols[function_type_id]
            .clone()
            .into_function()
            .unwrap();

        let checked_body = if let Some(body) = &function.body {
            let checked_body = self.visit_expr(body.clone(), ctx)?;
            let actual_return_type = checked_body.ty();

            if !self.unify(checked_function.return_type, actual_return_type, ctx) {
                return Err(Error::TypeMismatch {
                    location: function.location,
                    expected: vec![checked_function.return_type],
                    found: actual_return_type,
                });
            }
            Some(checked_body)
        } else {
            None
        };

        checked_function.body = checked_body.map(|x| self.program.exprs.alloc_item(x));

        Ok(checked_function)
    }

    fn visit_impl_methods_signature(
        &mut self,
        def_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let impl_node = ctx.definition(def_id).as_impl().cloned().unwrap();

        ctx.symbols.start_scope(ScopeKind::Impl);
        self.infcx.enter_context();

        let mut checked_generic_parameters = Vec::new();
        for generic_parameter in &impl_node.generic_parameters {
            let checked_generic_parameter =
                self.typecheck_generic_parameter(generic_parameter, ctx)?;
            let type_id = ctx
                .symbols
                .add_type_variable(ScopeKind::Impl, checked_generic_parameter)?;
            checked_generic_parameters.push(type_id);
        }

        let implementor_type_id = self.typecheck(&impl_node.ty, ctx)?;
        let implementor_poly_type_id = self.poly_of(implementor_type_id, ctx).unwrap();
        if ctx.symbols[implementor_type_id]
            .generic_parameters()
            .iter()
            .any(|&p| !ctx.symbols[p].is_type_variable())
        {
            return Err(Error::SpecializationNotAllowed {
                location: impl_node.location,
            });
        }

        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, implementor_type_id)?;

        let mut methods = Vec::new();

        for (generic_parameter, generic_arg) in ctx.symbols[implementor_poly_type_id]
            .generic_parameters()
            .iter()
            .zip_eq(ctx.symbols[implementor_type_id].generic_parameters())
        {
            if !self.unify(generic_parameter.clone(), generic_arg, ctx) {
                return Err(Error::TypeMismatch {
                    location: impl_node.location,
                    expected: vec![generic_parameter.clone()],
                    found: generic_arg,
                });
            }
        }

        for &function_id in &impl_node.body {
            ctx.push_node_id(NodeId::from(function_id));
            ctx.symbols.start_scope(ScopeKind::ImplMethod);
            self.infcx.enter_scope();

            let function = ctx.definition(function_id).as_function().cloned().unwrap();

            let mut checked_function = self.typecheck_function_signature(function, ctx)?;

            let ty = Type::Function(checked_function.clone());
            let type_id = ctx.symbols.add_type(None, checked_function.name, ty)?;
            ctx.symbols[type_id].as_function_mut().unwrap().type_id = type_id;
            checked_function.type_id = type_id;

            self.infcx.exit_scope();
            ctx.symbols.end_scope();
            ctx.pop_node_id();

            methods.push(CheckedDefinitionNode::Function(checked_function));
        }

        let checked_impl = CheckedImplNode {
            generic_parameters: checked_generic_parameters,
            ty: implementor_type_id,
            body: self.program.defs.alloc_items(methods),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            location: impl_node.location,
            comments: impl_node.comments,
        };

        self.infcx.exit_context();
        ctx.symbols.end_scope();

        let impl_id = self
            .program
            .defs
            .alloc_item(CheckedDefinitionNode::Impl(checked_impl));
        self.register_impl(impl_id, ctx)?;

        Ok(())
    }

    fn visit_impl_methods_body(
        &mut self,
        def_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let impl_node = ctx.definition(def_id).as_impl().cloned().unwrap();

        for &function_id in &impl_node.body {
            ctx.push_node_id(NodeId::from(function_id));
            ctx.symbols.start_scope(ScopeKind::ImplMethod);
            self.infcx.enter_scope();

            let function = ctx.definition(function_id).as_function().cloned().unwrap();

            let checked_function = self.typecheck_function_body(function, ctx)?;
            let type_id = checked_function.type_id;
            ctx.symbols[type_id] = Type::Function(checked_function);

            self.infcx.exit_scope();
            ctx.symbols.end_scope();
            ctx.pop_node_id();
        }

        Ok(())
    }
}
