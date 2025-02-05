mod definition;
mod expr;
mod stmt;
mod symbol_table;
mod r#type;
mod value;
mod variable;

mod artifact;
mod error;

use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
    rc::Rc,
};

pub use artifact::*;
pub use definition::*;
pub use expr::*;
use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use qed_common::Arena;
pub use r#type::*;
pub use stmt::*;
pub use symbol_table::*;
pub use value::*;
pub use variable::*;

pub use error::*;
use qed_ast::*;

use qed_parser::Parser;

use tracing::{debug, error, info, instrument, span, Level};

#[derive(Debug)]
pub struct TypeChecker<F: Clone + From<u32>, T: From<CheckedValueNode<F>>, C> {
    pub exprs: Arena<ExprId, CheckedExprNode<F>>,
    pub stmts: Arena<StmtId, CheckedStmtNode<F>>,
    _marker: std::marker::PhantomData<(T, C)>,
}

impl<F: Clone + From<u32>, T: From<CheckedValueNode<F>>, C> Index<ExprId> for TypeChecker<F, T, C> {
    type Output = CheckedExprNode<F>;

    fn index(&self, index: ExprId) -> &Self::Output {
        &self.exprs[index]
    }
}

impl<F: Clone + From<u32>, T: From<CheckedValueNode<F>>, C> Index<StmtId> for TypeChecker<F, T, C> {
    type Output = CheckedStmtNode<F>;

    fn index(&self, index: StmtId) -> &Self::Output {
        &self.stmts[index]
    }
}

impl<F: Clone + From<u32>, T: Clone + From<CheckedValueNode<F>>, C> TypeChecker<F, T, C> {
    pub fn new() -> Self {
        Self {
            exprs: Arena::new(),
            stmts: Arena::new(),
            _marker: std::marker::PhantomData,
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_program(
        &mut self,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<()> {
        symbols.load_modules(artifact.program.modules.iter());
        let mut colors = HashMap::new();
        artifact
            .program
            .dependency_graph
            .ts(&ModuleId::root(), &mut colors, &mut |&module_id| {
                let module = &artifact.program.modules[module_id];
                symbols.push_module(module_id);
                self.typecheck_module(module.data(), symbols, artifact)
                    .unwrap();
                symbols.pop_module();
            });
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_std_prelude_module(
        &mut self,
        module: &ModuleNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<()> {
        STD_PRELUDE_SCOPE_ID.set(symbols.current_scope_id().unwrap());
        for (ident, ty) in TYPE_MAPPING {
            symbols.add_type(None, ty.clone());
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_module(
        &mut self,
        module: &ModuleNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<()> {
        if module.is_std && module.is_self_prelude {
            self.typecheck_std_prelude_module(module, symbols, artifact)?;
        }

        for use_path in &module.uses {
            self.typecheck_use(use_path, symbols, artifact)?;
        }

        for &def_id in &module.definitions {
            let def = &artifact[def_id];
            self.typecheck_definition(def, symbols, artifact, Some(NodeType::Module))?;
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck(
        &mut self,
        ty: &UncheckedType,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<TypeId> {
        match ty {
            UncheckedType::Basic(IdentId::TYPE_BOOL) => Ok(BOOL_TYPE),
            UncheckedType::Basic(IdentId::TYPE_FELT) => Ok(FELT_TYPE),
            UncheckedType::Basic(name) => Ok(symbols
                .get_type_id(None, name.clone())
                .ok_or(Error::UnresolvedType)?),
            UncheckedType::Generic(name, generic_parameters) => {
                let type_id = symbols
                    .get_type_id(None, name.clone())
                    .ok_or(Error::UnresolvedType)?;

                let mut checked_generic_parameters = Vec::new();
                for generic_parameter in generic_parameters {
                    checked_generic_parameters.push(self.typecheck(
                        generic_parameter,
                        symbols,
                        artifact,
                    ));
                }

                let ty = &symbols[type_id];
                match ty {
                    Type::Struct(checked_struct) => {
                        // instantiate struct
                        todo!()
                    }
                    Type::Enum(checked_enum) => {
                        todo!()
                        // instantiate enum
                    }
                    _ => {
                        todo!()
                    }
                }
            }
            UncheckedType::Array(inner, size) => {
                let inner_ty = self.typecheck(inner, symbols, artifact)?;
                let scope_id = STD_PRELUDE_SCOPE_ID.get().cloned();
                let type_id = symbols.add_type(
                    scope_id,
                    Type::Array(CheckedArrayNode {
                        inner_ty,
                        size: size.clone(),
                        scope_id: scope_id.unwrap(),
                    }),
                );
                Ok(type_id)
            }
            UncheckedType::Unknown => Ok(UNKOWN_TYPE),
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_struct(
        &mut self,
        node: &StructNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedStructNode> {
        symbols.start_scope(ScopeKind::Struct);

        let mut generic_parameters = Vec::new();

        for &parameter in &node.generic_parameters {
            let type_id = symbols.add_type_variable(parameter);
            generic_parameters.push(type_id);
        }

        let mut checked_struct = CheckedStructNode {
            name: node.name.clone(),
            generic_parameters,
            fields: Vec::new(),
            implementations: Vec::new(),
            scope_id: symbols.current_scope_id().unwrap(),
            visibility: node.visibility,
        };

        for (field_name, field_type, visibility) in &node.fields {
            let field_type = self.typecheck(field_type, symbols, artifact)?;
            checked_struct
                .fields
                .push((field_name.clone(), field_type, *visibility));

            symbols.add_type_id(None, field_name.clone(), field_type);
        }

        let ty = Type::Struct(checked_struct.clone());
        symbols.add_type(symbols.parent_scope_id(), ty);

        symbols.end_scope();
        Ok(checked_struct)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_use(
        &mut self,
        use_path: &UsePath,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<()> {
        Ok(symbols.add_use(use_path)?)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_definition(
        &mut self,
        definition: &DefinitionNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedDefinitionNode> {
        match definition {
            DefinitionNode::Function(function) => Ok(CheckedDefinitionNode::Function({
                symbols.start_scope(ScopeKind::Function);
                let checked_function =
                    self.typecheck_function(function, symbols, artifact, parent_node_type)?;
                let ty = Type::Function(checked_function.clone());
                symbols.add_type(symbols.parent_scope_id(), ty);

                symbols.end_scope();

                checked_function
            })),
            DefinitionNode::Struct(r#struct) => Ok(CheckedDefinitionNode::Struct(
                self.typecheck_struct(r#struct, symbols, artifact, parent_node_type)?,
            )),
            DefinitionNode::Enum(r#enum) => Ok(CheckedDefinitionNode::Enum(self.typecheck_enum(
                r#enum,
                symbols,
                artifact,
                parent_node_type,
            )?)),
            DefinitionNode::Impl(r#impl) => Ok(CheckedDefinitionNode::Impl(self.typecheck_impl(
                r#impl,
                symbols,
                artifact,
                parent_node_type,
            )?)),
            DefinitionNode::Trait(r#trait) => Ok(CheckedDefinitionNode::Trait(
                self.typecheck_trait(r#trait, symbols, artifact, parent_node_type)?,
            )),
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_trait_method(
        &mut self,
        function: &FunctionNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedFunctionNode> {
        symbols.start_scope(ScopeKind::TraitMethod);
        symbols.add_type_id(None, IdentId::TYPE_SELF, UNKOWN_TYPE);

        let checked_function =
            self.typecheck_function(function, symbols, artifact, parent_node_type)?;
        let ty = Type::Function(checked_function.clone());
        symbols.add_type(None, ty);

        symbols.end_scope();
        Ok(checked_function)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_method(
        &mut self,
        function: &FunctionNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedFunctionNode> {
        symbols.start_scope(ScopeKind::ImplMethod);
        let checked_function =
            self.typecheck_function(function, symbols, artifact, parent_node_type)?;
        let ty = Type::Function(checked_function.clone());
        symbols.add_type(None, ty);

        symbols.end_scope();
        Ok(checked_function)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_function(
        &mut self,
        function: &FunctionNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedFunctionNode> {
        let current_scope_id = symbols.current_scope_id().unwrap();
        let mut generic_parameters = Vec::new();
        let mut parameters = Vec::new();

        for &generic_parameter in &function.generic_parameters {
            let type_id = symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for (parameter, mutable, parameter_type) in &function.parameters {
            let parameter_type = self.typecheck(parameter_type, symbols, artifact)?;
            let variable =
                CheckedVariable::new(parameter_type, *mutable, false, current_scope_id, None);
            symbols.declare_variable(parameter.clone(), variable);
            parameters.push((parameter.clone(), *mutable, parameter_type));
        }

        let expected_return_type = if let Some(ref ret) = function.return_type {
            Some(self.typecheck(ret, symbols, artifact)?)
        } else {
            None
        };

        let (checked_body, actual_return_type) = if let Some(body) = &function.body {
            let checked_body = self.typecheck_block(
                artifact[body.clone()].as_block().unwrap(),
                symbols,
                artifact,
                Some(function.node_type()),
            )?;

            let actual_return_type =
                checked_body
                    .stmts
                    .last()
                    .and_then(|stmt| match self[stmt.clone()] {
                        CheckedStmtNode::Return(CheckedReturnNode { ret }) => {
                            ret.as_ref().map(|(expr, ty)| ty.clone())
                        }
                        _ => None,
                    });

            (Some(checked_body), actual_return_type)
        } else {
            (None, expected_return_type)
        };

        if expected_return_type != actual_return_type {
            return Err(Error::TypeMismatch);
        }

        let checked_function = CheckedFunctionNode {
            name: function.name,
            parameters,
            generic_parameters,
            body: checked_body,
            return_type: expected_return_type,
            scope_id: current_scope_id,
            visibility: function.visibility,
        };

        Ok(checked_function)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_enum(
        &mut self,
        r#enum: &EnumNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedEnumNode> {
        symbols.start_scope(ScopeKind::Enum);
        let mut generic_parameters = Vec::new();

        for &generic_parameter in &r#enum.generic_parameters {
            let type_id = symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for variant in &r#enum.variants {
            match variant {
                EnumVariant::Basic(name) => todo!(),
                EnumVariant::Tuple(name, members) => todo!(),
                EnumVariant::Struct(name, fields) => todo!(),
            }
        }

        let checked_enum = CheckedEnumNode {
            name: todo!(),
            generic_parameters,
            variants: todo!(),
            scope_id: todo!(),
            implementations: Vec::new(),
            visibility: r#enum.visibility,
        };
        let ty = Type::Enum(checked_enum.clone());
        symbols.add_type(symbols.parent_scope_id(), ty);

        symbols.end_scope();
        Ok(checked_enum)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_impl_trait(
        &mut self,
        r#impl: &ImplNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedImplNode> {
        let (trait_scope, trait_type_id) = symbols.resolve_trait(r#impl.trait_name.unwrap())?;
        let (implementor_scope, implementor_type_id) = symbols.resolve_implementor(r#impl.ty)?;
        symbols.push_scope(trait_scope);
        symbols.start_scope(ScopeKind::Impl);

        symbols.add_type_id(None, IdentId::TYPE_SELF, implementor_type_id);
        symbols.add_type_id(
            None,
            symbols[implementor_type_id].key(),
            implementor_type_id,
        );
        symbols.add_type_id(None, IdentId::SELF, implementor_type_id);

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for &generic_parameter in &r#impl.generic_parameters {
            let type_id = symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for function in &r#impl.body {
            let f = artifact[function.clone()].as_function().unwrap();
            methods.push(self.typecheck_method(f, symbols, artifact, Some(r#impl.node_type()))?);
        }
        let checked_impl = CheckedImplNode {
            generic_parameters,
            trait_name: r#impl.trait_name,
            ty: r#impl.ty,
            body: methods,
            scope_id: symbols.current_scope_id().unwrap(),
        };
        let ty = Type::Impl(checked_impl.clone());
        // symbols.add_type(symbols.parent_scope_id(), ty);

        symbols.impl_trait_for_type(trait_type_id, implementor_type_id);

        symbols.end_scope();
        symbols.pop_scope();
        Ok(checked_impl)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_impl(
        &mut self,
        r#impl: &ImplNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedImplNode> {
        if r#impl.trait_name.is_some() {
            return self.typecheck_impl_trait(r#impl, symbols, artifact, parent_node_type);
        }

        let (implementor_scope, type_id) = symbols.resolve_implementor(r#impl.ty)?;
        symbols.push_scope(implementor_scope);
        symbols.start_scope(ScopeKind::Impl);

        symbols.add_type_id(None, IdentId::TYPE_SELF, type_id);
        symbols.add_type_id(None, IdentId::SELF, type_id);

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for &generic_parameter in &r#impl.generic_parameters {
            let type_id = symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for function in &r#impl.body {
            let f = artifact[function.clone()].as_function().unwrap();
            methods.push(self.typecheck_method(f, symbols, artifact, Some(r#impl.node_type()))?);
        }
        let checked_impl = CheckedImplNode {
            generic_parameters,
            trait_name: r#impl.trait_name,
            ty: r#impl.ty,
            body: methods,
            scope_id: symbols.current_scope_id().unwrap(),
        };
        let ty = Type::Impl(checked_impl.clone());
        // symbols.add_type(symbols.parent_scope_id(), ty);

        symbols.end_scope();
        symbols.pop_scope();
        Ok(checked_impl)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_trait(
        &mut self,
        r#trait: &TraitNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedTraitNode> {
        symbols.start_scope(ScopeKind::Trait);

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for &generic_parameter in &r#trait.generic_parameters {
            let type_id = symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for function in &r#trait.body {
            methods.push(self.typecheck_trait_method(
                function,
                symbols,
                artifact,
                Some(r#trait.node_type()),
            )?);
        }
        let checked_trait = CheckedTraitNode {
            generic_parameters,
            name: r#trait.name,
            body: methods,
            implementors: Vec::new(),
            scope_id: symbols.current_scope_id().unwrap(),
            visibility: r#trait.visibility,
        };
        let ty = Type::Trait(checked_trait.clone());
        symbols.add_type(symbols.parent_scope_id(), ty);

        symbols.end_scope();
        Ok(checked_trait)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_block(
        &mut self,
        body: &BlockNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedBlockNode> {
        symbols.start_scope(ScopeKind::Block);
        let mut new_stmts = Vec::with_capacity(body.stmts.len());
        for (i, stmt) in body.stmts.iter().enumerate() {
            let statement = &artifact[stmt.clone()];
            let checked_stmt =
                self.typecheck_stmt(statement, symbols, artifact, Some(body.node_type()))?;
            if parent_node_type == Some(NodeType::FunctionDef) {
                if let CheckedStmtNode::Return(CheckedReturnNode { ret }) = checked_stmt {
                    if i != body.stmts.len() - 1 {
                        return Err(Error::InvalidReturn);
                    }
                }
            }
            new_stmts.push(checked_stmt);
        }
        symbols.end_scope();
        Ok(CheckedBlockNode {
            stmts: self.stmts.alloc_items(new_stmts),
        })
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_stmt(
        &mut self,
        stmt: &StmtNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedStmtNode<F>> {
        match stmt {
            StmtNode::If(r#if) => Ok(CheckedStmtNode::If(self.typecheck_if(
                r#if,
                symbols,
                artifact,
                parent_node_type,
            )?)),
            StmtNode::While(r#while) => Ok(CheckedStmtNode::While(self.typecheck_while(
                r#while,
                symbols,
                artifact,
                parent_node_type,
            )?)),
            StmtNode::Block(block) => Ok(CheckedStmtNode::Block(self.typecheck_block(
                r#block,
                symbols,
                artifact,
                parent_node_type,
            )?)),
            StmtNode::Assignment(r#assignment) => Ok(CheckedStmtNode::Assignment(
                self.typecheck_assignment(r#assignment, symbols, artifact, parent_node_type)?,
            )),
            StmtNode::Variable(variable) => Ok(CheckedStmtNode::Variable(
                self.typecheck_variable(variable, symbols, artifact, parent_node_type)?,
            )),
            StmtNode::Definition(def_id) => {
                Ok(CheckedStmtNode::Definition(self.typecheck_definition(
                    &artifact[def_id.clone()],
                    symbols,
                    artifact,
                    parent_node_type,
                )?))
            }
            StmtNode::Expression(expr) => Ok(CheckedStmtNode::Expression(self.typecheck_expr(
                &artifact[expr.clone()],
                symbols,
                artifact,
                parent_node_type,
            )?)),
            StmtNode::Storage(storage_node) => {
                let offset = self.typecheck_expr(
                    &artifact[storage_node.offset.clone()],
                    symbols,
                    artifact,
                    Some(storage_node.node_type()),
                )?;
                let value = self.typecheck_expr(
                    &artifact[storage_node.value.clone()],
                    symbols,
                    artifact,
                    Some(storage_node.node_type()),
                )?;
                if offset.ty() != FELT_TYPE || value.ty() != FELT_TYPE {
                    return Err(Error::TypeMismatch);
                }
                Ok(CheckedStmtNode::Storage(CheckedStorageWriteNode {
                    offset: self.exprs.alloc_item(offset),
                    value: self.exprs.alloc_item(value),
                }))
            }
            StmtNode::Return(return_node) => {
                let current_scope_id = symbols.current_scope_id().unwrap();
                let parent_scope_id = symbols.parent_scope_id().unwrap();
                if symbols[current_scope_id].kind != ScopeKind::Block {
                    return Err(Error::InvalidReturn);
                }
                let valid_kinds = [
                    ScopeKind::Function,
                    ScopeKind::ImplMethod,
                    ScopeKind::TraitMethod,
                ];
                if !valid_kinds.contains(&symbols[parent_scope_id].kind) {
                    return Err(Error::InvalidReturn);
                }
                Ok(CheckedStmtNode::Return(self.typecheck_ret(
                    return_node,
                    symbols,
                    artifact,
                    parent_node_type,
                )?))
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_if(
        &mut self,
        r#if: &IfNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedIfNode> {
        let node_type = r#if.node_type();
        let checked_expr = self.typecheck_expr(
            &artifact[r#if.if_branch.predicate],
            symbols,
            artifact,
            Some(node_type),
        )?;
        if checked_expr.ty() != BOOL_TYPE {
            return Err(Error::TypeMismatch);
        }
        let checked_block = self.typecheck_block(
            artifact[r#if.if_branch.body].as_block().unwrap(),
            symbols,
            artifact,
            Some(node_type),
        )?;
        let if_branch = CheckedCase {
            predicate: self.exprs.alloc_item(checked_expr),
            type_id: BOOL_TYPE,
            body: checked_block,
        };

        let mut elseif_branch = Vec::with_capacity(r#if.elseif_branch.len());
        for branch in &r#if.elseif_branch {
            let checked_expr = self.typecheck_expr(
                &artifact[branch.predicate],
                symbols,
                artifact,
                Some(node_type),
            )?;
            if checked_expr.ty() != BOOL_TYPE {
                return Err(Error::TypeMismatch);
            }
            let checked_block = self.typecheck_block(
                artifact[branch.body].as_block().unwrap(),
                symbols,
                artifact,
                Some(node_type),
            )?;
            elseif_branch.push(CheckedCase {
                predicate: self.exprs.alloc_item(checked_expr),
                type_id: BOOL_TYPE,
                body: checked_block,
            });
        }

        let else_branch = if let Some(ref else_branch) = r#if.else_branch {
            Some(self.typecheck_block(
                artifact[else_branch.clone()].as_block().unwrap(),
                symbols,
                artifact,
                Some(node_type),
            )?)
        } else {
            None
        };

        Ok(CheckedIfNode {
            if_branch,
            elseif_branch,
            else_branch,
        })
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_while(
        &mut self,
        r#while: &WhileNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedWhileNode> {
        let node_type = r#while.node_type();
        let predicate = self.typecheck_expr(
            &artifact[r#while.predicate],
            symbols,
            artifact,
            Some(node_type),
        )?;
        if predicate.ty() != BOOL_TYPE {
            return Err(Error::TypeMismatch);
        }
        let checked_block = self.typecheck_block(
            artifact[r#while.body].as_block().unwrap(),
            symbols,
            artifact,
            Some(node_type),
        )?;
        Ok(CheckedWhileNode {
            predicate: self.exprs.alloc_item(predicate),
            type_id: BOOL_TYPE,
            body: checked_block,
        })
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_assignment(
        &mut self,
        r#assignment: &AssignmentNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedAssignmentNode> {
        let node_type = r#assignment.node_type();
        let checked_rhs = self.typecheck_expr(
            &artifact[r#assignment.value],
            symbols,
            artifact,
            Some(node_type),
        )?;
        let checked_lhs = self.typecheck_expr(
            &artifact[r#assignment.variable],
            symbols,
            artifact,
            Some(node_type),
        )?;

        let lhs_ty = checked_lhs.ty();

        if lhs_ty != checked_rhs.ty() {
            return Err(Error::TypeMismatch);
        }
        return Ok(CheckedAssignmentNode {
            variable: self.exprs.alloc_item(checked_lhs),
            operator: r#assignment.operator,
            value: self.exprs.alloc_item(checked_rhs),
            type_id: lhs_ty,
        });
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_ret(
        &mut self,
        r#ret: &ReturnNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedReturnNode> {
        let node_type = r#ret.node_type();
        let ret = if let Some(expr) = &r#ret.0 {
            let expr =
                self.typecheck_expr(&artifact[expr.clone()], symbols, artifact, Some(node_type))?;
            let ty = expr.ty();
            Some((self.exprs.alloc_item(expr), ty))
        } else {
            None
        };

        Ok(CheckedReturnNode { ret })
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_variable(
        &mut self,
        variable: &VariableNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedVariableNode> {
        let node_type = variable.node_type();
        let checked_expr = self.typecheck_expr(
            &artifact[variable.value],
            symbols,
            artifact,
            Some(node_type),
        )?;
        let ty = checked_expr.ty();
        if ty != self.typecheck(&variable.ty, symbols, artifact)? {
            return Err(Error::TypeMismatch);
        }
        let current_scope_id = symbols.current_scope_id().unwrap();
        symbols.declare_variable(
            variable.name.clone(),
            CheckedVariable::new(ty, variable.mutable, variable.cnst, current_scope_id, None),
        );
        let checked_variable = CheckedVariableNode {
            name: variable.name,
            ty,
            mutable: variable.mutable,
            cnst: variable.cnst,
            value: self.exprs.alloc_item(checked_expr),
            scope_id: current_scope_id,
        };
        Ok(checked_variable)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_expr(
        &mut self,
        expr: &ExprNode<F>,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedExprNode<F>> {
        match expr {
            ExprNode::Storage(storage_node) => {
                let offset = self.typecheck_expr(
                    &artifact[storage_node.offset],
                    symbols,
                    artifact,
                    Some(storage_node.node_type()),
                )?;
                if offset.ty() != FELT_TYPE {
                    return Err(Error::TypeMismatch);
                }
                return Ok(CheckedExprNode::Storage(CheckedStorageReadNode {
                    offset: self.exprs.alloc_item(offset),
                    type_id: FELT_TYPE,
                }));
            }
            ExprNode::Path(path) => {
                if let Some((type_id, scope_id)) = symbols.resolve_path(path) {
                    return Ok(CheckedExprNode::Path(CheckedPathNode {
                        name: path.target.clone(),
                        type_id,
                        scope_id,
                    }));
                } else {
                    return Err(Error::UnresolvedPath);
                }
            }
            ExprNode::Value(value_node) => match value_node {
                ValueNode::Felt(f) => Ok(CheckedExprNode::Value(CheckedValueNode::Felt(f.clone()))),
                ValueNode::Bool(b) => Ok(CheckedExprNode::Value(CheckedValueNode::Bool(b.clone()))),
                ValueNode::Array(size, arr) => {
                    if size != &arr.len() {
                        return Err(Error::TypeMismatch);
                    }

                    let mut inner_ty: Option<TypeId> = None;
                    let mut elements = Vec::with_capacity(arr.len());
                    for el in arr {
                        let checked_expr = self.typecheck_expr(
                            &artifact[el.clone()],
                            symbols,
                            artifact,
                            Some(value_node.node_type()),
                        )?;
                        if let Some(inner_ty) = inner_ty {
                            if checked_expr.ty() != inner_ty {
                                return Err(Error::TypeMismatch);
                            }
                        } else {
                            inner_ty = Some(checked_expr.ty());
                        }
                        elements.push(self.exprs.alloc_item(checked_expr));
                    }

                    let scope_id = STD_PRELUDE_SCOPE_ID.get().cloned();
                    let type_id = symbols.add_type(
                        scope_id,
                        Type::Array(CheckedArrayNode {
                            inner_ty: inner_ty.unwrap(),
                            size: size.clone(),
                            scope_id: scope_id.unwrap(),
                        }),
                    );

                    Ok(CheckedExprNode::Value(CheckedValueNode::Array(
                        type_id, elements,
                    )))
                }
                ValueNode::Struct(name, generic_parameters, data) => Ok({
                    let generic_parameters = generic_parameters
                        .into_iter()
                        .map(|x| self.typecheck(x, symbols, artifact).unwrap())
                        .collect::<Vec<_>>();
                    let type_id = symbols.get_type_id(None, name.clone()).unwrap();
                    let mut new_data = IndexMap::new();
                    if symbols[type_id].as_struct().unwrap().fields.len() != data.len() {
                        return Err(Error::TypeMismatch);
                    }
                    for (i, (k, v)) in data.iter().enumerate() {
                        let expr = self.typecheck_expr(
                            &artifact[v.clone()],
                            symbols,
                            artifact,
                            Some(value_node.node_type()),
                        )?;
                        let t = expr.ty();
                        new_data.insert(k.clone(), self.exprs.alloc_item(expr));

                        let checked_struct = symbols[type_id].as_struct().unwrap();

                        let (field_name, field_type, visibility) = checked_struct
                            .fields
                            .iter()
                            .find(|(field_name, field_type, _visibility)| field_name == k)
                            .unwrap();

                        if (k, t) != (field_name, *field_type) {
                            return Err(Error::TypeMismatch);
                        }
                    }
                    CheckedExprNode::Value(CheckedValueNode::Struct(type_id, new_data))
                }),
            },
            ExprNode::Binary(binary_node) => {
                let checked_lhs = self.typecheck_expr(
                    &artifact[binary_node.lhs],
                    symbols,
                    artifact,
                    Some(binary_node.node_type()),
                )?;
                let checked_rhs = self.typecheck_expr(
                    &artifact[binary_node.rhs],
                    symbols,
                    artifact,
                    Some(binary_node.node_type()),
                )?;

                let lhs_ty = checked_lhs.ty();
                if lhs_ty != checked_rhs.ty() {
                    return Err(Error::TypeMismatch);
                }

                let type_id = match binary_node.operator {
                    BinaryOperator::Add
                    | BinaryOperator::Sub
                    | BinaryOperator::Mul
                    | BinaryOperator::Div
                    | BinaryOperator::Mod
                    | BinaryOperator::BitShr
                    | BinaryOperator::BitShl
                    | BinaryOperator::BitAnd
                    | BinaryOperator::BitOr
                    | BinaryOperator::BitXor => lhs_ty,
                    BinaryOperator::And | BinaryOperator::Or => {
                        if lhs_ty != BOOL_TYPE {
                            return Err(Error::TypeMismatch);
                        }
                        BOOL_TYPE
                    }
                    BinaryOperator::Eq
                    | BinaryOperator::Neq
                    | BinaryOperator::Lt
                    | BinaryOperator::Lte
                    | BinaryOperator::Gt
                    | BinaryOperator::Gte => BOOL_TYPE,
                };

                Ok(CheckedExprNode::Binary(CheckedBinaryNode {
                    lhs: self.exprs.alloc_item(checked_lhs),
                    operator: binary_node.operator,
                    rhs: self.exprs.alloc_item(checked_rhs),
                    type_id,
                }))
            }
            ExprNode::Unary(unary_node) => {
                let checked_expr = self.typecheck_expr(
                    &artifact[unary_node.rhs],
                    symbols,
                    artifact,
                    Some(unary_node.node_type()),
                )?;
                let type_id = checked_expr.ty();

                if type_id != FELT_TYPE && type_id != BOOL_TYPE {
                    return Err(Error::TypeMismatch);
                }

                Ok(CheckedExprNode::Unary(CheckedUnaryNode {
                    operator: unary_node.operator,
                    rhs: self.exprs.alloc_item(checked_expr),
                    type_id,
                }))
            }
            ExprNode::Cast(cast_node) => {
                let src_expr = self.typecheck_expr(
                    &artifact[cast_node.value],
                    symbols,
                    artifact,
                    Some(cast_node.node_type()),
                )?;
                let src_type = src_expr.ty();
                let target_type = self.typecheck(&cast_node.target_type, symbols, artifact)?;

                if src_type == FELT_TYPE && target_type == BOOL_TYPE
                    || src_type == BOOL_TYPE && target_type == FELT_TYPE
                {
                    return Ok(CheckedExprNode::Cast(CheckedCastNode {
                        value: self.exprs.alloc_item(src_expr),
                        target_type,
                    }));
                } else {
                    return Err(Error::TypeMismatch);
                };
            }
            ExprNode::Call(call_node) => {
                let variable = self.typecheck_expr(
                    &artifact[call_node.variable],
                    symbols,
                    artifact,
                    Some(call_node.node_type()),
                )?;
                let ty = variable.ty();
                let f = symbols[ty].as_function().unwrap().clone();
                let mut args = Vec::new();
                let receiver = if let Some(receiver) = call_node.receiver {
                    let expr = self.typecheck_expr(
                        &artifact[receiver],
                        symbols,
                        artifact,
                        Some(expr.node_type()),
                    )?;
                    if expr.ty() != f.parameters[0].2 {
                        return Err(Error::FunctionParameterMismatch);
                    }
                    Some(self.exprs.alloc_item(expr))
                } else {
                    None
                };
                let offset: usize = if receiver.is_some() { 1 } else { 0 };
                for (i, arg) in call_node.args.iter().cloned().enumerate() {
                    let type_arg = self.typecheck_expr(
                        &artifact[arg.clone()],
                        symbols,
                        artifact,
                        Some(call_node.node_type()),
                    )?;
                    if type_arg.ty() != f.parameters[i + offset].2 {
                        return Err(Error::FunctionParameterMismatch);
                    }
                    args.push(type_arg);
                }

                return Ok(CheckedExprNode::Call(CheckedCallNode {
                    variable: self.exprs.alloc_item(variable),
                    receiver,
                    generic_parameters: f.generic_parameters.clone(),
                    args: self.exprs.alloc_items(args),
                    type_id: f.return_type.unwrap_or(VOID_TYPE),
                }));
            }
            ExprNode::IndexAccess(index_access_node) => {
                let checked_expr = self.typecheck_expr(
                    &artifact[index_access_node.value],
                    symbols,
                    artifact,
                    Some(index_access_node.node_type()),
                )?;

                let type_id = checked_expr.ty();
                let ty = &symbols[type_id];

                Ok(CheckedExprNode::IndexAccess(CheckedIndexAccessNode {
                    value: self.exprs.alloc_item(checked_expr),
                    index: index_access_node.index,
                    type_id: ty.as_array().unwrap().inner_ty.clone(),
                }))
            }
            ExprNode::MemberAccess(member_access_node) => {
                let checked_expr = self.typecheck_expr(
                    &artifact[member_access_node.value],
                    symbols,
                    artifact,
                    Some(member_access_node.node_type()),
                )?;
                let type_id = checked_expr.ty();
                let ty = &symbols[type_id];
                let checked_struct_node = ty.as_struct().unwrap();
                for (field_name, field_type, visibility) in &checked_struct_node.fields {
                    let checked_struct_node_parent_scope_id =
                        symbols[checked_struct_node.scope_id].parent;

                    if field_name == &member_access_node.field {
                        return Ok(CheckedExprNode::MemberAccess(CheckedMemberAccessNode {
                            value: self.exprs.alloc_item(checked_expr),
                            field: field_name.clone(),
                            type_id: field_type.clone(),
                        }));
                    }
                }

                let type_id = symbols
                    .resolve_method(type_id, member_access_node.field)
                    .ok_or(Error::UnresolvedMember)?;

                return Ok(CheckedExprNode::MemberAccess(CheckedMemberAccessNode {
                    value: self.exprs.alloc_item(checked_expr),
                    field: member_access_node.field,
                    type_id,
                }));
            }
        }
    }
}
