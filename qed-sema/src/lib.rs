mod definition;
mod expr;
mod module;
mod program;
mod stmt;
mod symbol_table;
mod r#type;
mod value;
mod variable;

pub mod error;

use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
    rc::Rc,
};

pub use definition::*;
pub use expr::*;
use qed_common::{Arena, Hook};
pub use r#type::*;
pub use stmt::*;
pub use symbol_table::*;
pub use value::*;
pub use variable::*;

pub use error::*;
use qed_ast::*;

use qed_parser::Parser;

#[derive(Debug)]
pub struct TypeChecker<F: Clone, C> {
    pub exprs: Arena<ExprId, CheckedExprNode<F>>,
    pub stmts: Arena<StmtId, CheckedStmtNode<F>>,
    _marker: std::marker::PhantomData<C>,
}

#[derive(Debug)]
pub struct ParsingArtifact<F: Clone, C> {
    pub parser: Parser<F, C>,
    pub program: Program,
}

macro_rules! impl_index {
    ($index_type:ty, $output_type:ty, $field:ident) => {
        impl<F: Clone, C> Index<$index_type> for ParsingArtifact<F, C> {
            type Output = $output_type;
            fn index(&self, index: $index_type) -> &Self::Output {
                &self.parser.$field[index]
            }
        }

        impl<F: Clone, C> IndexMut<$index_type> for ParsingArtifact<F, C> {
            fn index_mut(&mut self, index: $index_type) -> &mut Self::Output {
                &mut self.parser.$field[index]
            }
        }
    };
}

impl_index!(ExprId, ExprNode<F>, exprs);
impl_index!(StmtId, StmtNode<F>, stmts);
impl_index!(IdentId, Ident, interner);

impl<F: Clone, C> Index<ExprId> for TypeChecker<F, C> {
    type Output = CheckedExprNode<F>;

    fn index(&self, index: ExprId) -> &Self::Output {
        &self.exprs[index]
    }
}

impl<F: Clone, C> Index<StmtId> for TypeChecker<F, C> {
    type Output = CheckedStmtNode<F>;

    fn index(&self, index: StmtId) -> &Self::Output {
        &self.stmts[index]
    }
}

impl<F: Clone, C> ParsingArtifact<F, C> {
    pub fn new(parser: Parser<F, C>, program: Program) -> Self {
        Self { parser, program }
    }
}

impl<F: Clone, C> TypeChecker<F, C> {
    pub fn new() -> Self {
        Self {
            exprs: Arena::new(),
            stmts: Arena::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn typecheck_program(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
    ) -> Result<()> {
        let mut module_registry = HashMap::new();
        artifact.program.dependency_graph.dfs(
            &artifact.program.root_file_id,
            None,
            &mut |file_id, parent_file_id, hook| {
                let module = artifact.program.modules.get(&file_id).unwrap();
                if hook == Hook::PreVisit {
                    if let Some(&module_id) = module_registry.get(&file_id) {
                        symbols.start_existing_module(module_id);
                    } else {
                        symbols.start_module(module.module_name, module.file_id);
                    }
                } else if hook == Hook::PostVisit {
                    symbols.end_module();
                } else {
                    if !module_registry.contains_key(file_id) {
                        module_registry.insert(file_id, symbols.current_module_id().unwrap());
                    }
                }
            },
        );

        let mut colors = HashMap::new();
        artifact.program.dependency_graph.ts(
            &artifact.program.root_file_id,
            &mut colors,
            &mut |file_id| {
                let module = artifact.program.modules.get(&file_id).unwrap();
                let module_id = module_registry.get(file_id).unwrap().clone();
                symbols.push_scope(symbols[module_id].scope_id);
                symbols.push_module(module_id);
                self.typecheck_module(symbols, artifact, module).unwrap();
                symbols.pop_scope();
                symbols.pop_module();
            },
        );

        self.print_module(symbols, artifact, ModuleId(0));

        Ok(())
    }

    pub fn typecheck_module(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        // module_name: &IdentId,
        module: &RawModule,
    ) -> Result<()> {
        // symbols.start_module(module_name.clone(), module.file_id);
        if symbols.current_module_id().unwrap() == ModuleId(0) {
            symbols.add_type_id(None, IdentId::TYPE_UNKNOWN, UNKOWN_TYPE);
            symbols.add_type_id(None, IdentId::TYPE_BOOL, BOOL_TYPE);
            symbols.add_type_id(None, IdentId::TYPE_FELT, FELT_TYPE);
            symbols.add_type_id(None, IdentId::TYPE_VOID, VOID_TYPE);
        }
        for use_path in &module.uses {
            // TODO: this wont be able to resolve
            self.typecheck_use(symbols, artifact, use_path)?;
        }

        for definition in &module.definitions {
            self.typecheck_definition(symbols, artifact, definition)?;
        }
        // symbols.end_module();
        Ok(())
    }

    pub fn print_module(
        &self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        id: ModuleId,
    ) {
        println!("Symbol Table Hierarchy:");
        self.print_module_hierarchy(symbols, artifact, id, 0);
    }

    pub fn print_module_hierarchy(
        &self,
        symbols: &SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        module_id: ModuleId,
        indent: usize,
    ) {
        let module = &symbols[module_id];
        let indent_str = "  ".repeat(indent);

        println!("{}Module: {:?}", indent_str, module.name);

        self.print_scope_hierarchy(symbols, artifact, module.scope_id, indent + 1);

        for &child_module_id in &module.children {
            self.print_module_hierarchy(symbols, artifact, child_module_id, indent + 1);
        }
    }

    pub fn print_scope_hierarchy(
        &self,
        symbols: &SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        scope_id: ScopeId,
        indent: usize,
    ) {
        let scope = &symbols[scope_id];
        let indent_str = "  ".repeat(indent);

        println!("{}Scope: {:?}", indent_str, scope.kind);

        if !scope.variables.is_empty() {
            println!("{}  Variables:", indent_str);
            for (ident, var) in &scope.variables {
                println!(
                    "{}    {:?}",
                    indent_str,
                    artifact.parser.interner[ident.clone()],
                    // symbols[var.ty]
                );
            }
        }

        if !scope.types.is_empty() {
            println!("{}  Types:", indent_str);
            for (type_key, type_id) in &scope.types {
                println!(
                    "{}    {:?}",
                    indent_str,
                    artifact.parser.interner[type_key.id.clone()],
                    // symbols[type_id.clone()]
                );
            }
        }

        // if !scope.uses.is_empty() {
        //     println!("{}  Uses:", indent_str);
        //     for (ident, type_id) in &scope.uses {
        //         println!("{}    {:?}: {:?}", indent_str, ident, type_id);
        //     }
        // }

        for &child_scope_id in &scope.children {
            self.print_scope_hierarchy(symbols, artifact, child_scope_id, indent + 1);
        }
    }

    pub fn typecheck(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        ty: &UncheckedType,
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
                        symbols,
                        artifact,
                        generic_parameter,
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
            UncheckedType::Array(inner, size) => todo!(),
            UncheckedType::Unknown => Ok(UNKOWN_TYPE),
        }
    }

    pub fn typecheck_struct(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        node: &StructNode,
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
            impls: HashMap::new(),
            scope_id: symbols.current_scope_id().unwrap(),
        };

        for (field_name, field_type) in &node.fields {
            let filed_type = self.typecheck(symbols, artifact, field_type)?;
            checked_struct.fields.push((field_name.clone(), filed_type));
        }

        let ty = Type::Struct(checked_struct.clone());
        symbols.add_type(symbols.parent_scope_id(), ty);

        symbols.end_scope();
        Ok(checked_struct)
    }

    pub fn typecheck_use(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        use_path: &UsePath,
    ) -> Result<()> {
        Ok(symbols.add_use(use_path)?)
    }

    pub fn typecheck_definition(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        definition: &DefinitionNode,
    ) -> Result<CheckedDefinitionNode> {
        match definition {
            DefinitionNode::Function(function) => Ok(CheckedDefinitionNode::Function(
                self.typecheck_function(symbols, artifact, function)?,
            )),
            DefinitionNode::Struct(r#struct) => Ok(CheckedDefinitionNode::Struct(
                self.typecheck_struct(symbols, artifact, r#struct)?,
            )),
            DefinitionNode::Enum(r#enum) => Ok(CheckedDefinitionNode::Enum(
                self.typecheck_enum(symbols, artifact, r#enum)?,
            )),
            DefinitionNode::Impl(r#impl) => Ok(CheckedDefinitionNode::Impl(
                self.typecheck_impl(symbols, artifact, r#impl)?,
            )),
        }
    }

    fn typecheck_method(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        function: &FunctionNode,
    ) -> Result<CheckedFunctionNode> {
        symbols.start_scope(ScopeKind::ImplMethod);
        let mut generic_parameters = Vec::new();
        let mut parameters = Vec::new();

        let current_scope_id = symbols.current_scope_id().unwrap();

        for &parameter in &function.generic_parameters {
            let type_id = symbols.add_type_variable(parameter);
            generic_parameters.push(type_id);
        }

        for (parameter, mutable, parameter_type) in &function.parameters {
            let parameter_type = self.typecheck(symbols, artifact, parameter_type)?;
            let variable =
                CheckedVariable::new(parameter_type, *mutable, false, current_scope_id, None);
            symbols.define_variable(parameter.clone(), variable);
            parameters.push((parameter.clone(), *mutable, parameter_type));
        }

        let checked_function = CheckedFunctionNode {
            name: function.name,
            parameters,
            generic_parameters,
            body: self.typecheck_block(symbols, artifact, &function.body)?,
            return_type: if let Some(ref ret) = function.return_type {
                Some(self.typecheck(symbols, artifact, ret)?)
            } else {
                None
            },
            scope_id: current_scope_id,
        };
        let ty = Type::Function(checked_function.clone());
        symbols.add_type(None, ty);

        symbols.end_scope();
        Ok(checked_function)
    }

    fn typecheck_function(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        function: &FunctionNode,
    ) -> Result<CheckedFunctionNode> {
        symbols.start_scope(ScopeKind::Function);
        let current_scope_id = symbols.current_scope_id().unwrap();
        let mut generic_parameters = Vec::new();
        let mut parameters = Vec::new();

        for &generic_parameter in &function.generic_parameters {
            let type_id = symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for (parameter, mutable, parameter_type) in &function.parameters {
            let parameter_type = self.typecheck(symbols, artifact, parameter_type)?;
            let variable =
                CheckedVariable::new(parameter_type, *mutable, false, current_scope_id, None);
            symbols.define_variable(parameter.clone(), variable);
            parameters.push((parameter.clone(), *mutable, parameter_type));
        }

        let return_type = if let Some(ref ret) = function.return_type {
            Some(self.typecheck(symbols, artifact, ret)?)
        } else {
            None
        };

        let checked_function = CheckedFunctionNode {
            name: function.name.clone(),
            parameters,
            generic_parameters,
            body: self.typecheck_block(symbols, artifact, &function.body)?,
            return_type,
            scope_id: current_scope_id,
        };

        let ty = Type::Function(checked_function.clone());
        symbols.add_type(symbols.parent_scope_id(), ty);

        symbols.end_scope();
        Ok(checked_function)
    }

    fn typecheck_enum(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        r#enum: &EnumNode,
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
        };
        let ty = Type::Enum(checked_enum.clone());
        symbols.add_type(symbols.parent_scope_id(), ty);

        symbols.end_scope();
        Ok(checked_enum)
    }

    fn typecheck_impl(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        r#impl: &ImplNode,
    ) -> Result<CheckedImplNode> {
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
            methods.push(self.typecheck_method(symbols, artifact, function)?);
        }
        let checked_impl = CheckedImplNode {
            generic_parameters,
            ty: r#impl.ty,
            body: methods,
        };
        let ty = Type::Impl(checked_impl.clone());
        symbols.add_type(symbols.parent_scope_id(), ty);

        symbols.end_scope();
        symbols.pop_scope();
        Ok(checked_impl)
    }

    fn typecheck_block(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        body: &BlockNode,
    ) -> Result<CheckedBlockNode> {
        symbols.start_scope(ScopeKind::Block);
        let mut new_stmts = Vec::with_capacity(body.stmts.len());
        for stmt in body.stmts.iter() {
            let statement = &artifact[stmt.clone()];
            new_stmts.push(self.typecheck_stmt(symbols, artifact, statement)?);
        }
        symbols.end_scope();
        Ok(CheckedBlockNode {
            stmts: self.stmts.alloc_items(new_stmts),
        })
    }

    fn typecheck_stmt(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        stmt: &StmtNode<F>,
    ) -> Result<CheckedStmtNode<F>> {
        match stmt {
            StmtNode::If(r#if) => Ok(CheckedStmtNode::If(
                self.typecheck_if(symbols, artifact, r#if)?,
            )),
            StmtNode::While(r#while) => Ok(CheckedStmtNode::While(
                self.typecheck_while(symbols, artifact, r#while)?,
            )),
            StmtNode::Block(block) => Ok(CheckedStmtNode::Block(
                self.typecheck_block(symbols, artifact, r#block)?,
            )),
            StmtNode::Assignment(r#assignment) => Ok(CheckedStmtNode::Assignment(
                self.typecheck_assignment(symbols, artifact, r#assignment)?,
            )),
            StmtNode::Variable(variable) => Ok(CheckedStmtNode::Variable(
                self.typecheck_variable(symbols, artifact, variable)?,
            )),
            StmtNode::Definition(definition) => Ok(CheckedStmtNode::Definition(
                self.typecheck_definition(symbols, artifact, definition)?,
            )),
            StmtNode::Expression(expr) => Ok(CheckedStmtNode::Expression(
                self.typecheck_expr(symbols, artifact, expr)?,
            )),
            StmtNode::Return(return_node) => Ok(CheckedStmtNode::Return(self.typecheck_ret(
                symbols,
                artifact,
                return_node,
            )?)),
        }
    }

    fn typecheck_if(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        r#if: &IfNode,
    ) -> Result<CheckedIfNode> {
        let checked_expr =
            self.typecheck_expr(symbols, artifact, &artifact[r#if.if_branch.predicate])?;
        if checked_expr.ty() != BOOL_TYPE {
            return Err(Error::TypeMismatch);
        }
        let checked_block = self.typecheck_block(symbols, artifact, &r#if.if_branch.body)?;
        let if_branch = CheckedCase {
            predicate: self.exprs.alloc_item(checked_expr),
            type_id: BOOL_TYPE,
            body: checked_block,
        };

        let mut elseif_branch = Vec::with_capacity(r#if.elseif_branch.len());
        for branch in &r#if.elseif_branch {
            let checked_expr =
                self.typecheck_expr(symbols, artifact, &artifact[branch.predicate])?;
            if checked_expr.ty() != BOOL_TYPE {
                return Err(Error::TypeMismatch);
            }
            let checked_block = self.typecheck_block(symbols, artifact, &branch.body)?;
            elseif_branch.push(CheckedCase {
                predicate: self.exprs.alloc_item(checked_expr),
                type_id: BOOL_TYPE,
                body: checked_block,
            });
        }

        let else_branch = if let Some(ref else_branch) = r#if.else_branch {
            Some(self.typecheck_block(symbols, artifact, else_branch)?)
        } else {
            None
        };

        Ok(CheckedIfNode {
            if_branch,
            elseif_branch,
            else_branch,
        })
    }

    fn typecheck_while(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        r#while: &WhileNode,
    ) -> Result<CheckedWhileNode> {
        let predicate = self.typecheck_expr(symbols, artifact, &artifact[r#while.predicate])?;
        if predicate.ty() != BOOL_TYPE {
            return Err(Error::TypeMismatch);
        }
        let checked_block = self.typecheck_block(symbols, artifact, &r#while.body)?;
        Ok(CheckedWhileNode {
            predicate: self.exprs.alloc_item(predicate),
            type_id: BOOL_TYPE,
            body: checked_block,
        })
    }

    fn typecheck_assignment(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        r#assignment: &AssignmentNode,
    ) -> Result<CheckedAssignmentNode> {
        let checked_rhs = self.typecheck_expr(symbols, artifact, &artifact[r#assignment.value])?;
        let checked_lhs =
            self.typecheck_expr(symbols, artifact, &artifact[r#assignment.variable])?;

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

    fn typecheck_ret(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        r#ret: &ReturnNode,
    ) -> Result<CheckedReturnNode> {
        let ret = if let Some(expr) = &r#ret.0 {
            let expr = self.typecheck_expr(symbols, artifact, &artifact[expr.clone()])?;
            let ty = expr.ty();
            Some((self.exprs.alloc_item(expr), ty))
        } else {
            None
        };

        Ok(CheckedReturnNode { ret })
    }

    fn typecheck_variable(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        variable: &VariableNode,
    ) -> Result<CheckedVariableNode> {
        let checked_expr = self.typecheck_expr(symbols, artifact, &artifact[variable.value])?;
        let ty = checked_expr.ty();
        if ty != self.typecheck(symbols, artifact, &variable.ty)? {
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

    fn typecheck_expr(
        &mut self,
        symbols: &mut SymbolTable<CheckedValueNode<F>>,
        artifact: &ParsingArtifact<F, C>,
        expr: &ExprNode<F>,
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
                ValueNode::Array(_, _) => todo!(),
                ValueNode::Struct(name, generic_parameters, data) => Ok({
                    let generic_parameters = generic_parameters
                        .into_iter()
                        .map(|x| self.typecheck(symbols, artifact, x).unwrap())
                        .collect::<Vec<_>>();
                    let type_key = TypeKey::new(name.clone(), generic_parameters);
                    let type_id = symbols.get_type_id(None, type_key).unwrap();

                    let mut new_data = HashMap::new();
                    for (k, v) in data {
                        let expr = self.typecheck_expr(symbols, artifact, &artifact[v.clone()])?;
                        new_data.insert(k.clone(), self.exprs.alloc_item(expr));
                    }
                    CheckedExprNode::Value(CheckedValueNode::Struct(type_id, new_data))
                }),
            },
            ExprNode::Binary(binary_node) => {
                let checked_lhs =
                    self.typecheck_expr(symbols, artifact, &artifact[binary_node.lhs])?;
                let checked_rhs =
                    self.typecheck_expr(symbols, artifact, &artifact[binary_node.rhs])?;

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
                    self.typecheck_expr(symbols, artifact, &artifact[unary_node.rhs])?;
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
            ExprNode::Call(call_node) => {
                let expr = self.typecheck_expr(symbols, artifact, &artifact[call_node.variable])?;
                let ty = expr.ty();
                if let Type::Function(f) = symbols[ty].clone() {
                    let mut args = Vec::new();
                    let receiver = if let Some(receiver) = call_node.receiver {
                        let expr = self.typecheck_expr(symbols, artifact, &artifact[receiver])?;
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
                            self.typecheck_expr(symbols, artifact, &artifact[arg.clone()])?;
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
            ExprNode::IndexAccess(index_access_node) => todo!(),
            ExprNode::MemberAccess(member_access_node) => {
                let checked_expr =
                    self.typecheck_expr(symbols, artifact, &artifact[member_access_node.value])?;
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

                        if let Some(type_id) = symbols
                            .resolve_method(checked_struct_node.scope_id, member_access_node.field)
                        {
                            return Ok(CheckedExprNode::MemberAccess(CheckedMemberAccessNode {
                                value: self.exprs.alloc_item(checked_expr),
                                field: member_access_node.field,
                                type_id,
                            }));
                        }

                        return Err(Error::UnresolvedMember);
                    }
                    _ => {
                        return Err(Error::UnresolvedMember);
                    }
                }
            }
        }
    }
}
