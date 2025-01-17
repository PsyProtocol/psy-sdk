mod artifact;
mod definition;
mod expr;
mod stmt;
mod symbol_table;
mod r#type;
mod value;
mod variable;

mod error;

use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
    rc::Rc,
};

pub use artifact::*;
pub use definition::*;
pub use expr::*;
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
pub struct TypeChecker<F: Clone, T: From<CheckedValueNode<F>>, C> {
    pub exprs: Arena<ExprId, CheckedExprNode<F>>,
    pub stmts: Arena<StmtId, CheckedStmtNode<F>>,
    _marker: std::marker::PhantomData<(T, C)>,
}

impl<F: Clone, T: From<CheckedValueNode<F>>, C> Index<ExprId> for TypeChecker<F, T, C> {
    type Output = CheckedExprNode<F>;

    fn index(&self, index: ExprId) -> &Self::Output {
        &self.exprs[index]
    }
}

impl<F: Clone, T: From<CheckedValueNode<F>>, C> Index<StmtId> for TypeChecker<F, T, C> {
    type Output = CheckedStmtNode<F>;

    fn index(&self, index: StmtId) -> &Self::Output {
        &self.stmts[index]
    }
}

static STD_PRELUDE_SCOPE_ID: OnceCell<ScopeId> = OnceCell::new();

impl<F: Clone, T: From<CheckedValueNode<F>>, C> TypeChecker<F, T, C> {
    pub fn new() -> Self {
        Self {
            exprs: Arena::new(),
            stmts: Arena::new(),
            _marker: std::marker::PhantomData,
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_init(
        &mut self,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<()> {
        for module in artifact.program.modules.iter() {
            let data = module.data();
            symbols.modules.push(Module {
                name: data.name.clone(),
                id: module.id(),
                scope_id: ScopeId(module.id().into()),
                kind: ModuleKind::File {
                    file_id: data.file_id,
                },
                parent: module.parent(),
                children: module.children().to_vec(),
            });
            symbols.scopes.push(Scope {
                kind: ScopeKind::Module,
                parent: module.parent().map(|x| ScopeId(x.into())),
                children: module
                    .children()
                    .into_iter()
                    .map(|&x| ScopeId(x.into()))
                    .collect(),
                variables: HashMap::with_capacity(10),
                types: HashMap::new(),
            })
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_program(
        &mut self,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<()> {
        self.typecheck_init(symbols, artifact)?;
        let mut colors = HashMap::new();
        artifact
            .program
            .dependency_graph
            .ts(&ModuleId::root(), &mut colors, &mut |&module_id| {
                let module = &artifact.program.modules[module_id];
                symbols.push_scope(symbols[module_id].scope_id);
                symbols.push_module(module_id);
                self.typecheck_module(module.data(), symbols, artifact)
                    .unwrap();
                symbols.pop_scope();
                symbols.pop_module();
            });

        // self.print_module(ModuleId::root(), symbols, artifact);

        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_std_prelude_module(
        &mut self,
        module: &RawModule,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<()> {
        STD_PRELUDE_SCOPE_ID.set(symbols.current_scope_id().unwrap());
        for (ident, ty) in TYPE_MAPPING {
            symbols.add_type_id(None, ident.clone(), *ty);
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_module(
        &mut self,
        module: &RawModule,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<()> {
        if module.is_std && module.is_self_prelude {
            self.typecheck_std_prelude_module(module, symbols, artifact)?;
        }

        for use_path in &module.uses {
            self.typecheck_use(use_path, symbols, artifact)?;
        }

        for definition in &module.definitions {
            self.typecheck_definition(definition, symbols, artifact)?;
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
        };

        for (field_name, field_type) in &node.fields {
            let field_type = self.typecheck(field_type, symbols, artifact)?;
            checked_struct.fields.push((field_name.clone(), field_type));

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
    ) -> Result<CheckedDefinitionNode> {
        match definition {
            DefinitionNode::Function(function) => Ok(CheckedDefinitionNode::Function({
                symbols.start_scope(ScopeKind::Function);
                let checked_function = self.typecheck_function(function, symbols, artifact)?;
                let ty = Type::Function(checked_function.clone());
                symbols.add_type(symbols.parent_scope_id(), ty);

                symbols.end_scope();

                checked_function
            })),
            DefinitionNode::Struct(r#struct) => Ok(CheckedDefinitionNode::Struct(
                self.typecheck_struct(r#struct, symbols, artifact)?,
            )),
            DefinitionNode::Enum(r#enum) => Ok(CheckedDefinitionNode::Enum(
                self.typecheck_enum(r#enum, symbols, artifact)?,
            )),
            DefinitionNode::Impl(r#impl) => Ok(CheckedDefinitionNode::Impl(
                self.typecheck_impl(r#impl, symbols, artifact)?,
            )),
            DefinitionNode::Trait(r#trait) => Ok(CheckedDefinitionNode::Trait(
                self.typecheck_trait(r#trait, symbols, artifact)?,
            )),
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_trait_method(
        &mut self,
        function: &FunctionNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<CheckedFunctionNode> {
        symbols.start_scope(ScopeKind::TraitMethod);
        symbols.add_type_id(None, IdentId::TYPE_SELF, UNKOWN_TYPE);

        let checked_function = self.typecheck_function(function, symbols, artifact)?;
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
    ) -> Result<CheckedFunctionNode> {
        symbols.start_scope(ScopeKind::ImplMethod);
        let checked_function = self.typecheck_function(function, symbols, artifact)?;
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
            symbols.define_variable(parameter.clone(), variable);
            parameters.push((parameter.clone(), *mutable, parameter_type));
        }

        let expected_return_type = if let Some(ref ret) = function.return_type {
            Some(self.typecheck(ret, symbols, artifact)?)
        } else {
            None
        };

        let checked_body = if let Some(body) = &function.body {
            let checked_body = self.typecheck_block(body, symbols, artifact)?;

            if expected_return_type.is_some() {
                for stmt in checked_body.stmts.iter() {
                    if let CheckedStmtNode::Return(CheckedReturnNode { ret }) = &self[*stmt] {
                        if ret.map(|(_, type_id)| type_id) != expected_return_type {
                            return Err(Error::TypeMismatch);
                        }
                    }
                }
            }

            Some(checked_body)
        } else {
            None
        };

        let checked_function = CheckedFunctionNode {
            name: function.name,
            parameters,
            generic_parameters,
            body: checked_body,
            return_type: expected_return_type,
            scope_id: current_scope_id,
        };

        Ok(checked_function)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_enum(
        &mut self,
        r#enum: &EnumNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
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
    ) -> Result<CheckedImplNode> {
        let (trait_scope, trait_type_id) = symbols.resolve_trait(r#impl.trait_name.unwrap())?;
        let (implementor_scope, implementor_type_id) = symbols.resolve_implementor(r#impl.ty)?;
        symbols.push_scope(trait_scope);
        symbols.start_scope(ScopeKind::Impl);

        symbols.add_type_id(None, IdentId::TYPE_SELF, implementor_type_id);
        symbols.add_type_id(None, IdentId::SELF, implementor_type_id);

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for &generic_parameter in &r#impl.generic_parameters {
            let type_id = symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for function in &r#impl.body {
            methods.push(self.typecheck_method(function, symbols, artifact)?);
        }
        let checked_impl = CheckedImplNode {
            generic_parameters,
            trait_name: r#impl.trait_name,
            ty: r#impl.ty,
            body: methods,
            scope_id: symbols.current_scope_id().unwrap(),
        };
        let ty = Type::Impl(checked_impl.clone());
        symbols.add_type(symbols.parent_scope_id(), ty);

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
    ) -> Result<CheckedImplNode> {
        if r#impl.trait_name.is_some() {
            return self.typecheck_impl_trait(r#impl, symbols, artifact);
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
            methods.push(self.typecheck_method(function, symbols, artifact)?);
        }
        let checked_impl = CheckedImplNode {
            generic_parameters,
            trait_name: r#impl.trait_name,
            ty: r#impl.ty,
            body: methods,
            scope_id: symbols.current_scope_id().unwrap(),
        };
        let ty = Type::Impl(checked_impl.clone());
        symbols.add_type(symbols.parent_scope_id(), ty);

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
    ) -> Result<CheckedTraitNode> {
        symbols.start_scope(ScopeKind::Trait);

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for &generic_parameter in &r#trait.generic_parameters {
            let type_id = symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for function in &r#trait.body {
            methods.push(self.typecheck_trait_method(function, symbols, artifact)?);
        }
        let checked_trait = CheckedTraitNode {
            generic_parameters,
            name: r#trait.name,
            body: methods,
            implementors: Vec::new(),
            scope_id: symbols.current_scope_id().unwrap(),
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
    ) -> Result<CheckedBlockNode> {
        symbols.start_scope(ScopeKind::Block);
        let mut new_stmts = Vec::with_capacity(body.stmts.len());
        for stmt in body.stmts.iter() {
            let statement = &artifact[stmt.clone()];
            new_stmts.push(self.typecheck_stmt(statement, symbols, artifact)?);
        }
        symbols.end_scope();
        Ok(CheckedBlockNode {
            stmts: self.stmts.alloc_items(new_stmts),
        })
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_stmt(
        &mut self,
        stmt: &StmtNode<F>,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<CheckedStmtNode<F>> {
        match stmt {
            StmtNode::If(r#if) => Ok(CheckedStmtNode::If(
                self.typecheck_if(r#if, symbols, artifact)?,
            )),
            StmtNode::While(r#while) => Ok(CheckedStmtNode::While(
                self.typecheck_while(r#while, symbols, artifact)?,
            )),
            StmtNode::Block(block) => Ok(CheckedStmtNode::Block(
                self.typecheck_block(r#block, symbols, artifact)?,
            )),
            StmtNode::Assignment(r#assignment) => Ok(CheckedStmtNode::Assignment(
                self.typecheck_assignment(r#assignment, symbols, artifact)?,
            )),
            StmtNode::Variable(variable) => Ok(CheckedStmtNode::Variable(
                self.typecheck_variable(variable, symbols, artifact)?,
            )),
            StmtNode::Definition(definition) => Ok(CheckedStmtNode::Definition(
                self.typecheck_definition(definition, symbols, artifact)?,
            )),
            StmtNode::Expression(expr) => Ok(CheckedStmtNode::Expression(
                self.typecheck_expr(expr, symbols, artifact)?,
            )),
            StmtNode::Return(return_node) => Ok(CheckedStmtNode::Return(self.typecheck_ret(
                return_node,
                symbols,
                artifact,
            )?)),
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_if(
        &mut self,
        r#if: &IfNode,
        symbols: &mut SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) -> Result<CheckedIfNode> {
        let checked_expr =
            self.typecheck_expr(&artifact[r#if.if_branch.predicate], symbols, artifact)?;
        if checked_expr.ty() != BOOL_TYPE {
            return Err(Error::TypeMismatch);
        }
        let checked_block = self.typecheck_block(&r#if.if_branch.body, symbols, artifact)?;
        let if_branch = CheckedCase {
            predicate: self.exprs.alloc_item(checked_expr),
            type_id: BOOL_TYPE,
            body: checked_block,
        };

        let mut elseif_branch = Vec::with_capacity(r#if.elseif_branch.len());
        for branch in &r#if.elseif_branch {
            let checked_expr =
                self.typecheck_expr(&artifact[branch.predicate], symbols, artifact)?;
            if checked_expr.ty() != BOOL_TYPE {
                return Err(Error::TypeMismatch);
            }
            let checked_block = self.typecheck_block(&branch.body, symbols, artifact)?;
            elseif_branch.push(CheckedCase {
                predicate: self.exprs.alloc_item(checked_expr),
                type_id: BOOL_TYPE,
                body: checked_block,
            });
        }

        let else_branch = if let Some(ref else_branch) = r#if.else_branch {
            Some(self.typecheck_block(else_branch, symbols, artifact)?)
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
    ) -> Result<CheckedWhileNode> {
        let predicate = self.typecheck_expr(&artifact[r#while.predicate], symbols, artifact)?;
        if predicate.ty() != BOOL_TYPE {
            return Err(Error::TypeMismatch);
        }
        let checked_block = self.typecheck_block(&r#while.body, symbols, artifact)?;
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
    ) -> Result<CheckedAssignmentNode> {
        let checked_rhs = self.typecheck_expr(&artifact[r#assignment.value], symbols, artifact)?;
        let checked_lhs =
            self.typecheck_expr(&artifact[r#assignment.variable], symbols, artifact)?;

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
    ) -> Result<CheckedReturnNode> {
        let ret = if let Some(expr) = &r#ret.0 {
            let expr = self.typecheck_expr(&artifact[expr.clone()], symbols, artifact)?;
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
    ) -> Result<CheckedVariableNode> {
        let checked_expr = self.typecheck_expr(&artifact[variable.value], symbols, artifact)?;
        let ty = checked_expr.ty();
        if ty != self.typecheck(&variable.ty, symbols, artifact)? {
            return Err(Error::TypeMismatch);
        }
        let current_scope_id = symbols.current_scope_id().unwrap();
        symbols.define_variable(
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
    ) -> Result<CheckedExprNode<F>> {
        match expr {
            ExprNode::Path(PathNode(ident)) => {
                let scope_id = symbols.current_scope_id().unwrap();
                if let Some(variable) = symbols.get_variable(None, &ident) {
                    Ok(CheckedExprNode::Path(CheckedPathNode {
                        name: ident.clone(),
                        type_id: variable.ty,
                        scope_id: variable.scope_id,
                    }))
                } else if let Some(type_id) = symbols.get_type_id(None, ident.clone()) {
                    return Ok(CheckedExprNode::Path(CheckedPathNode {
                        name: ident.clone(),
                        type_id,
                        scope_id,
                    }));
                } else {
                    return Err(Error::UnresolvedVariable);
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
                        let checked_expr =
                            self.typecheck_expr(&artifact[el.clone()], symbols, artifact)?;
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
                    let type_key = TypeKey::new(name.clone(), generic_parameters, vec![]);
                    let type_id = symbols.get_type_id(None, type_key).unwrap();
                    let mut new_data = HashMap::new();
                    if let Type::Struct(checked_struct) = &symbols[type_id] {
                        if checked_struct.fields.len() != data.len() {
                            return Err(Error::TypeMismatch);
                        }
                    } else {
                        unreachable!()
                    }
                    for (i, (k, v)) in data.iter().enumerate() {
                        let expr = self.typecheck_expr(&artifact[v.clone()], symbols, artifact)?;
                        let t = expr.ty();
                        new_data.insert(k.clone(), self.exprs.alloc_item(expr));

                        if let Type::Struct(checked_struct) = &symbols[type_id] {
                            if Some(&(k.clone(), t))
                                != checked_struct
                                    .fields
                                    .iter()
                                    .find(|(field_name, field_type)| field_name == k)
                            {
                                return Err(Error::TypeMismatch);
                            }
                        } else {
                            unreachable!()
                        }
                    }
                    CheckedExprNode::Value(CheckedValueNode::Struct(type_id, new_data))
                }),
            },
            ExprNode::Binary(binary_node) => {
                let checked_lhs =
                    self.typecheck_expr(&artifact[binary_node.lhs], symbols, artifact)?;
                let checked_rhs =
                    self.typecheck_expr(&artifact[binary_node.rhs], symbols, artifact)?;

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
                let checked_expr =
                    self.typecheck_expr(&artifact[unary_node.rhs], symbols, artifact)?;
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
                let src_expr =
                    self.typecheck_expr(&artifact[cast_node.value], symbols, artifact)?;
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
                let expr = self.typecheck_expr(&artifact[call_node.variable], symbols, artifact)?;
                let ty = expr.ty();
                if let Type::Function(f) = symbols[ty].clone() {
                    let mut args = Vec::new();
                    let receiver = if let Some(receiver) = call_node.receiver {
                        let expr = self.typecheck_expr(&artifact[receiver], symbols, artifact)?;
                        if expr.ty() != f.parameters[0].2 {
                            return Err(Error::FunctionParameterMismatch);
                        }
                        Some(self.exprs.alloc_item(expr))
                    } else {
                        None
                    };
                    let offset: usize = if receiver.is_some() { 1 } else { 0 };
                    for (i, arg) in call_node.args.iter().cloned().enumerate() {
                        let type_arg =
                            self.typecheck_expr(&artifact[arg.clone()], symbols, artifact)?;
                        if type_arg.ty() != f.parameters[i + offset].2 {
                            return Err(Error::FunctionParameterMismatch);
                        }
                        args.push(type_arg);
                    }

                    return Ok(CheckedExprNode::Call(CheckedCallNode {
                        variable: self.exprs.alloc_item(expr),
                        receiver,
                        generic_parameters: f.generic_parameters.clone(),
                        args: self.exprs.alloc_items(args),
                        type_id: f.return_type.unwrap_or(VOID_TYPE),
                    }));
                } else {
                    return Err(Error::InvalidFunctionCall);
                }
            }
            ExprNode::IndexAccess(index_access_node) => {
                let checked_expr =
                    self.typecheck_expr(&artifact[index_access_node.value], symbols, artifact)?;

                let type_id = checked_expr.ty();
                let ty = &symbols[type_id];

                match ty {
                    Type::Array(checked_array) => {
                        Ok(CheckedExprNode::IndexAccess(CheckedIndexAccessNode {
                            value: self.exprs.alloc_item(checked_expr),
                            index: index_access_node.index,
                            type_id: checked_array.inner_ty.clone(),
                        }))
                    }
                    _ => unreachable!(),
                }
            }
            ExprNode::MemberAccess(member_access_node) => {
                let checked_expr =
                    self.typecheck_expr(&artifact[member_access_node.value], symbols, artifact)?;
                let type_id = checked_expr.ty();
                let ty = &symbols[type_id];
                match ty {
                    Type::Struct(checked_struct_node) => {
                        for (field_name, field_type) in &checked_struct_node.fields {
                            if field_name == &member_access_node.field {
                                return Ok(CheckedExprNode::MemberAccess(
                                    CheckedMemberAccessNode {
                                        value: self.exprs.alloc_item(checked_expr),
                                        field: field_name.clone(),
                                        type_id: field_type.clone(),
                                    },
                                ));
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
                    _ => unreachable!(),
                }
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn print_module(&self, id: ModuleId, symbols: &SymbolTable<T>, artifact: &Artifact<F, C>) {
        println!("Symbol Table Hierarchy:");
        self.print_module_hierarchy(id, 0, symbols, artifact);
    }

    #[instrument(level = "debug", skip_all)]
    pub fn print_module_hierarchy(
        &self,
        module_id: ModuleId,
        indent: usize,
        symbols: &SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) {
        let module = &symbols[module_id];
        let indent_str = "  ".repeat(indent);

        println!("{}Module: {:?}", indent_str, module.name);

        self.print_scope_hierarchy(module.scope_id, indent + 1, symbols, artifact);

        for &child_module_id in &module.children {
            self.print_module_hierarchy(child_module_id, indent + 1, symbols, artifact);
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn print_scope_hierarchy(
        &self,
        scope_id: ScopeId,
        indent: usize,
        symbols: &SymbolTable<T>,
        artifact: &Artifact<F, C>,
    ) {
        let scope = &symbols[scope_id];
        let indent_str = "  ".repeat(indent);

        println!("{}Scope: {:?}", indent_str, scope.kind);

        if !scope.variables.is_empty() {
            println!("{}  Variables:", indent_str);
            for (ident, var) in &scope.variables {
                println!(
                    "{}    {:?}: {:?}",
                    indent_str,
                    artifact[ident.clone()],
                    symbols[var.ty]
                );
            }
        }

        if !scope.types.is_empty() {
            println!("{}  Types:", indent_str);
            for (type_key, type_id) in &scope.types {
                println!(
                    "{}    {:?}: {:?}",
                    indent_str,
                    artifact[type_key.id.clone()],
                    symbols[type_id.clone()]
                );
            }
        }

        for &child_scope_id in &scope.children {
            self.print_scope_hierarchy(child_scope_id, indent + 1, symbols, artifact);
        }
    }
}
