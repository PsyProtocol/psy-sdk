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

pub use definition::*;
pub use expr::*;
use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use qed_common::{Arena, Graph, Tree, TreeNode};
pub use r#type::*;
pub use stmt::*;
pub use symbol_table::*;
pub use value::*;
pub use variable::*;

pub use error::*;
use qed_ast::*;

use qed_parser::Parser;

use tracing::{debug, error, info, instrument, span, Level};

pub struct TypeCheckerVisitorContext<F: Clone + From<u32>, C> {
    path_stack: Vec<NodeId>,
    pub program: Program<F>,
    pub symbols: SymbolTable<F>,
    _marker: std::marker::PhantomData<(F, C)>,
}

impl<F: Clone + From<u32>, C> TypeCheckerVisitorContext<F, C> {
    pub fn new(program: Program<F>) -> Self {
        TypeCheckerVisitorContext {
            path_stack: vec![],
            program,
            symbols: SymbolTable::new(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<F: Clone + From<u32>, C> VisitorContext<F, C> for TypeCheckerVisitorContext<F, C> {
    type Expr = ExprNode<F>;

    type Stmt = StmtNode;

    type Definition = DefinitionNode;

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
        unimplemented!()
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

    fn expression(&self, expr_id: ExprId) -> &Self::Expr {
        &self.program.exprs[expr_id]
    }

    fn statement(&self, stmt_id: StmtId) -> &Self::Stmt {
        &self.program.stmts[stmt_id]
    }

    fn definition(&self, def_id: DefId) -> &Self::Definition {
        &self.program.defs[def_id]
    }

    fn insert_definition(&mut self, definition: Self::Definition, pos: InsertPosition) {
        unimplemented!()
    }

    fn alloc_expression(&mut self, expr: Self::Expr) -> ExprId {
        unimplemented!()
    }

    fn alloc_statement(&mut self, stmt: Self::Stmt) -> StmtId {
        unimplemented!()
    }

    fn alloc_definition(&mut self, definition: Self::Definition) -> DefId {
        unimplemented!()
    }

    fn replace_definition(&mut self, def_id: DefId, definition: Self::Definition) {
        unimplemented!()
    }

    fn replace_statement(&mut self, stmt_id: StmtId, statement: Self::Stmt) {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct TypeChecker<F: Clone + From<u32>, C> {
    pub exprs: Arena<ExprId, CheckedExprNode<F>>,
    pub stmts: Arena<StmtId, CheckedStmtNode>,
    pub defs: Arena<DefId, CheckedDefinitionNode>,
    _marker: std::marker::PhantomData<C>,
}

impl<F: Clone + From<u32>, C> AstVisitor<F, C> for TypeChecker<F, C> {
    type Expr = ExprNode<F>;

    type Stmt = StmtNode;

    type Definition = DefinitionNode;

    type ExprResult = CheckedExprNode<F>;

    type StmtResult = CheckedStmtNode;

    type DefinitionResult = CheckedDefinitionNode;

    type Context = TypeCheckerVisitorContext<F, C>;

    type Error = Error;

    fn visit_use(
        &mut self,
        use_path: &UsePath,
        ctx: &mut Self::Context,
    ) -> std::result::Result<(), Self::Error> {
        Ok(ctx.symbols.add_use(use_path)?)
    }

    fn visit_path(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let path_node = ctx.expression(node).as_path().cloned().unwrap();
        if let Some((type_id, scope_id)) = ctx.symbols.resolve_path(&path_node) {
            return Ok(CheckedExprNode::Path(CheckedPathNode {
                name: path_node.target,
                type_id,
                scope_id,
            }));
        } else {
            return Err(Error::UnresolvedPath);
        }
    }

    fn visit_index_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let index_access_node = ctx.expression(node).as_index_access().cloned().unwrap();
        let checked_expr = self.visit_expr(index_access_node.target, ctx)?;

        let type_id = checked_expr.ty();
        let ty = &ctx.symbols[type_id];

        Ok(CheckedExprNode::IndexAccess(CheckedIndexAccessNode {
            value: self.exprs.alloc_item(checked_expr),
            index: index_access_node.index,
            type_id: ty.as_array().unwrap().inner_ty.clone(),
        }))
    }

    fn visit_member_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let member_access_node = ctx.expression(node).as_member_access().cloned().unwrap();
        let checked_expr = self.visit_expr(member_access_node.target, ctx)?;
        let type_id = checked_expr.ty();
        let ty = &ctx.symbols[type_id];
        let checked_struct_node = ty.as_struct().unwrap();

        if let Some((field_type, visibility)) =
            checked_struct_node.fields.get(&member_access_node.field)
        {
            assert!(
                visibility.is_public()
                    || self.typecheck_member_access(member_access_node.target, ctx)
            );
            return Ok(CheckedExprNode::MemberAccess(CheckedMemberAccessNode {
                value: self.exprs.alloc_item(checked_expr),
                field: member_access_node.field.clone(),
                type_id: field_type.clone(),
            }));
        }

        let type_id = ctx
            .symbols
            .resolve_method(type_id, member_access_node.field)
            .ok_or(Error::UnresolvedMember)?;
        let visibility = ctx.symbols[type_id].visibility();
        assert!(
            visibility.is_public() || self.typecheck_member_access(member_access_node.target, ctx)
        );
        return Ok(CheckedExprNode::MemberAccess(CheckedMemberAccessNode {
            value: self.exprs.alloc_item(checked_expr),
            field: member_access_node.field,
            type_id,
        }));
    }

    fn visit_storage_read(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let storage_node = ctx.expression(node).as_storage().cloned().unwrap();
        let offset = self.visit_expr(storage_node.offset, ctx)?;
        if offset.ty() != FELT_TYPE {
            return Err(Error::TypeMismatch);
        }
        return Ok(CheckedExprNode::Storage(CheckedStorageReadNode {
            offset: self.exprs.alloc_item(offset),
            type_id: FELT_TYPE,
        }));
    }

    fn visit_context(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let context_node = ctx.expression(node).as_context().cloned().unwrap();
        match context_node {
            ContextNode::GetUserId => {
                return Ok(CheckedExprNode::Context(CheckedContextNode::GetUserId {
                    type_id: FELT_TYPE,
                }));
            }
            ContextNode::GetContractId => {
                return Ok(CheckedExprNode::Context(
                    CheckedContextNode::GetContractId { type_id: FELT_TYPE },
                ));
            }
            ContextNode::GetCheckpointId => {
                return Ok(CheckedExprNode::Context(
                    CheckedContextNode::GetCheckpointId { type_id: FELT_TYPE },
                ));
            }
            ContextNode::GetLastNonce => {
                return Ok(CheckedExprNode::Context(CheckedContextNode::GetLastNonce {
                    type_id: FELT_TYPE,
                }));
            }
            ContextNode::GetUserPublicKeyHash => {
                let scope_id = ScopeId::prelude();
                let type_id = ctx.symbols.add_type(
                    Some(scope_id),
                    Type::Array(CheckedArrayNode {
                        inner_ty: FELT_TYPE,
                        size: 4,
                        scope_id: scope_id,
                    }),
                );
                return Ok(CheckedExprNode::Context(
                    CheckedContextNode::GetUserPublicKeyHash { type_id },
                ));
            }
            ContextNode::GetStateHashAt { slot_index } => {
                let slot_index = self.visit_expr(slot_index, ctx)?;
                if slot_index.ty() != FELT_TYPE {
                    return Err(Error::TypeMismatch);
                }
                let scope_id = ScopeId::prelude();
                let type_id = ctx.symbols.add_type(
                    Some(scope_id),
                    Type::Array(CheckedArrayNode {
                        inner_ty: FELT_TYPE,
                        size: 4,
                        scope_id: scope_id,
                    }),
                );
                return Ok(CheckedExprNode::Context(
                    CheckedContextNode::GetStateHashAt {
                        slot_index: self.exprs.alloc_item(slot_index),
                        type_id,
                    },
                ));
            }
            ContextNode::GetOtherContractStateHashAt {
                contract_state_tree_height,
                contract_id,
                slot_index,
            } => {
                let contract_state_tree_height =
                    self.visit_expr(contract_state_tree_height, ctx)?;
                let contract_id = self.visit_expr(contract_id, ctx)?;
                let slot_index = self.visit_expr(slot_index, ctx)?;
                if contract_state_tree_height.ty() != FELT_TYPE
                    || contract_id.ty() != FELT_TYPE
                    || slot_index.ty() != FELT_TYPE
                {
                    return Err(Error::TypeMismatch);
                }
                let scope_id = ScopeId::prelude();
                let type_id = ctx.symbols.add_type(
                    Some(scope_id),
                    Type::Array(CheckedArrayNode {
                        inner_ty: FELT_TYPE,
                        size: 4,
                        scope_id: scope_id,
                    }),
                );

                return Ok(CheckedExprNode::Context(
                    CheckedContextNode::GetOtherContractStateHashAt {
                        contract_state_tree_height: self
                            .exprs
                            .alloc_item(contract_state_tree_height),
                        contract_id: self.exprs.alloc_item(contract_id),
                        slot_index: self.exprs.alloc_item(slot_index),
                        type_id,
                    },
                ));
            }
            ContextNode::GetOtherUserContractStateHashAt {
                contract_state_tree_height,
                user_id,
                contract_id,
                slot_index,
            } => {
                let contract_state_tree_height =
                    self.visit_expr(contract_state_tree_height, ctx)?;
                let user_id = self.visit_expr(user_id, ctx)?;
                let contract_id = self.visit_expr(contract_id, ctx)?;
                let slot_index = self.visit_expr(slot_index, ctx)?;
                if contract_state_tree_height.ty() != FELT_TYPE
                    || user_id.ty() != FELT_TYPE
                    || contract_id.ty() != FELT_TYPE
                    || slot_index.ty() != FELT_TYPE
                {
                    return Err(Error::TypeMismatch);
                }
                let scope_id = ScopeId::prelude();
                let type_id = ctx.symbols.add_type(
                    Some(scope_id),
                    Type::Array(CheckedArrayNode {
                        inner_ty: FELT_TYPE,
                        size: 4,
                        scope_id: scope_id,
                    }),
                );

                return Ok(CheckedExprNode::Context(
                    CheckedContextNode::GetOtherUserContractStateHashAt {
                        contract_state_tree_height: self
                            .exprs
                            .alloc_item(contract_state_tree_height),
                        user_id: self.exprs.alloc_item(user_id),
                        contract_id: self.exprs.alloc_item(contract_id),
                        slot_index: self.exprs.alloc_item(slot_index),
                        type_id,
                    },
                ));
            }
            ContextNode::CSetStateHashAt {
                slot_index,
                new_value,
            } => {
                let slot_index = self.visit_expr(slot_index, ctx)?;
                let new_value = self.visit_expr(new_value, ctx)?;
                let scope_id = ScopeId::prelude();
                let type_id = ctx.symbols.add_type(
                    Some(scope_id),
                    Type::Array(CheckedArrayNode {
                        inner_ty: FELT_TYPE,
                        size: 4,
                        scope_id: scope_id,
                    }),
                );

                if slot_index.ty() != FELT_TYPE || new_value.ty() != type_id {
                    return Err(Error::TypeMismatch);
                }

                return Ok(CheckedExprNode::Context(
                    CheckedContextNode::CSetStateHashAt {
                        slot_index: self.exprs.alloc_item(slot_index),
                        new_value: self.exprs.alloc_item(new_value),
                        type_id,
                    },
                ));
            }
        }
    }

    fn visit_assert(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        let assert_node = ctx.expression(node).as_assert().cloned().unwrap();
        let checked_lhs = self.visit_expr(assert_node.left, ctx)?;

        if checked_lhs.ty() != BOOL_TYPE {
            return Err(Error::TypeMismatch);
        }

        Ok(CheckedExprNode::Assert(CheckedAssertNode {
            left: self.exprs.alloc_item(checked_lhs),
            message: assert_node.message,
        }))
    }

    fn visit_assert_eq(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        let assert_eq_node = ctx.expression(node).as_assert_eq().cloned().unwrap();
        let checked_lhs = self.visit_expr(assert_eq_node.left, ctx)?;
        let checked_rhs = self.visit_expr(assert_eq_node.right, ctx)?;

        if checked_lhs.ty() != checked_rhs.ty() {
            return Err(Error::TypeMismatch);
        }

        Ok(CheckedExprNode::AssertEq(CheckedAssertEqNode {
            left: self.exprs.alloc_item(checked_lhs),
            right: self.exprs.alloc_item(checked_rhs),
            message: assert_eq_node.message,
        }))
    }

    fn visit_value(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let value_node = ctx.expression(node).as_value().cloned().unwrap();
        match value_node {
            ValueNode::Felt(f) => Ok(CheckedExprNode::Value(CheckedValueNode::Felt(f.clone()))),
            ValueNode::Bool(b) => Ok(CheckedExprNode::Value(CheckedValueNode::Bool(b.clone()))),
            ValueNode::String(s) => Ok(CheckedExprNode::Value(CheckedValueNode::String(s.clone()))),
            ValueNode::Array(size, arr) => {
                if size != arr.len() {
                    return Err(Error::TypeMismatch);
                }

                let mut inner_ty: Option<TypeId> = None;
                let mut elements = Vec::with_capacity(arr.len());
                for el in arr {
                    // TODO: remove clone
                    let checked_expr = self.visit_expr(el, ctx)?;
                    if let Some(inner_ty) = inner_ty {
                        if checked_expr.ty() != inner_ty {
                            return Err(Error::TypeMismatch);
                        }
                    } else {
                        inner_ty = Some(checked_expr.ty());
                    }
                    elements.push(self.exprs.alloc_item(checked_expr));
                }

                let scope_id = ScopeId::prelude();
                let type_array = Type::Array(CheckedArrayNode {
                    inner_ty: inner_ty.unwrap(),
                    size: size.clone(),
                    scope_id,
                });
                let type_id = match ctx.symbols.get_type_id(Some(scope_id), type_array.key()){
                    Some(type_id) => type_id,
                    None => {
                        ctx.symbols.add_type(Some(scope_id), type_array)
                    }
                };

                Ok(CheckedExprNode::Value(CheckedValueNode::Array(
                    type_id, elements,
                )))
            }
            ValueNode::Struct(name, generic_parameters, data) => Ok({
                let generic_parameters = generic_parameters
                    .into_iter()
                    .map(|x| self.typecheck(&x, ctx).unwrap())
                    .collect::<Vec<_>>();
                let type_id = ctx.symbols.get_type_id(None, name).unwrap();
                let mut new_data = IndexMap::new();
                if ctx.symbols[type_id].as_struct().unwrap().fields.len() != data.len() {
                    return Err(Error::TypeMismatch);
                }
                for (i, (k, v)) in data.iter().enumerate() {
                    let expr = self.visit_expr(v.clone(), ctx)?;
                    let t = expr.ty();
                    new_data.insert(k.clone(), self.exprs.alloc_item(expr));

                    let checked_struct = ctx.symbols[type_id].as_struct().unwrap();

                    let (field_name, (field_type, visibility)) = checked_struct
                        .fields
                        .iter()
                        .find(|(field_name, (field_type, _visibility))| *field_name == k)
                        .unwrap();

                    if (k, t) != (field_name, *field_type) {
                        return Err(Error::TypeMismatch);
                    }
                }
                CheckedExprNode::Value(CheckedValueNode::Struct(type_id, new_data))
            }),
        }
    }

    fn visit_binary(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let binary_node = ctx.expression(node).as_binary().cloned().unwrap();
        let checked_lhs = self.visit_expr(binary_node.lhs, ctx)?;
        let checked_rhs = self.visit_expr(binary_node.rhs, ctx)?;

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

    fn visit_unary(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let unary_node = ctx.expression(node).as_unary().cloned().unwrap();
        let checked_expr = self.visit_expr(unary_node.rhs, ctx)?;
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

    fn visit_call(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let call_node = ctx.expression(node).as_call().cloned().unwrap();
        let variable = self.visit_expr(call_node.variable, ctx)?;
        let ty = variable.ty();
        // TODO: remove clone
        let f = ctx.symbols[ty].as_function().unwrap().clone();
        let mut args = Vec::new();
        // TODO: add member call
        let receiver = if let Some(receiver) = call_node.receiver {
            let expr = self.visit_expr(receiver, ctx)?;
            if expr.ty() != f.parameters[0].2 {
                return Err(Error::FunctionParameterMismatch);
            }
            Some(self.exprs.alloc_item(expr))
        } else {
            None
        };
        let offset: usize = if receiver.is_some() { 1 } else { 0 };
        for (i, arg) in call_node.args.iter().enumerate() {
            let type_arg = self.visit_expr(arg.clone(), ctx)?;
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

    fn visit_cast(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let cast_node = ctx.expression(node).as_cast().cloned().unwrap();
        let src_expr = self.visit_expr(cast_node.value, ctx)?;
        let src_type = src_expr.ty();
        let target_type = self.typecheck(&cast_node.target_type, ctx)?;

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

    fn visit_if(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let if_node = ctx.statement(node).as_if().cloned().unwrap();
        let checked_expr = self.visit_expr(if_node.if_branch.predicate, ctx)?;
        if checked_expr.ty() != BOOL_TYPE {
            return Err(Error::TypeMismatch);
        }
        let checked_block = self.visit_block(if_node.if_branch.body, ctx)?;
        let if_branch = CheckedCase {
            predicate: self.exprs.alloc_item(checked_expr),
            type_id: BOOL_TYPE,
            body: self.stmts.alloc_item(checked_block),
        };

        let mut elseif_branch = Vec::with_capacity(if_node.elseif_branch.len());
        for branch in &if_node.elseif_branch {
            let checked_expr = self.visit_expr(branch.predicate, ctx)?;
            if checked_expr.ty() != BOOL_TYPE {
                return Err(Error::TypeMismatch);
            }
            let checked_block = self.visit_block(branch.body, ctx)?;
            elseif_branch.push(CheckedCase {
                predicate: self.exprs.alloc_item(checked_expr),
                type_id: BOOL_TYPE,
                body: self.stmts.alloc_item(checked_block),
            });
        }

        let else_branch = if let Some(else_branch) = if_node.else_branch {
            let block = self.visit_block(else_branch, ctx)?;
            Some(self.stmts.alloc_item(block))
        } else {
            None
        };

        Ok(CheckedStmtNode::If(CheckedIfNode {
            if_branch,
            elseif_branch,
            else_branch,
        }))
    }

    fn visit_while(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let while_node = ctx.statement(node).as_while().cloned().unwrap();
        let predicate = self.visit_expr(while_node.predicate, ctx)?;
        if predicate.ty() != BOOL_TYPE {
            return Err(Error::TypeMismatch);
        }
        let checked_block = self.visit_block(while_node.body, ctx)?;
        Ok(CheckedStmtNode::While(CheckedWhileNode {
            predicate: self.exprs.alloc_item(predicate),
            type_id: BOOL_TYPE,
            body: self.stmts.alloc_item(checked_block),
        }))
    }

    fn visit_block(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Block);
        // TODO: remove clone
        let block = ctx.statement(node).as_block().cloned().unwrap();
        let mut new_stmts = Vec::with_capacity(block.stmts.len());
        for (i, stmt) in block.stmts.iter().enumerate() {
            let checked_stmt = self.visit_stmt(stmt.clone(), ctx)?;
            if ctx.parent_node_type() == NodeType::FunctionDef {
                if let CheckedStmtNode::Return(CheckedReturnNode { ret }) = checked_stmt {
                    if i != block.stmts.len() - 1 {
                        return Err(Error::InvalidReturn);
                    }
                }
            }
            new_stmts.push(checked_stmt);
        }
        ctx.symbols.end_scope();
        Ok(CheckedStmtNode::Block(CheckedBlockNode {
            stmts: self.stmts.alloc_items(new_stmts),
        }))
    }

    fn visit_assignment(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let assignment_node = ctx.statement(node).as_assignment().cloned().unwrap();
        let checked_rhs = self.visit_expr(assignment_node.value, ctx)?;
        let checked_lhs = self.visit_expr(assignment_node.variable, ctx)?;

        let lhs_ty = checked_lhs.ty();

        if lhs_ty != checked_rhs.ty() {
            return Err(Error::TypeMismatch);
        }

        return Ok(CheckedStmtNode::Assignment(CheckedAssignmentNode {
            variable: self.exprs.alloc_item(checked_lhs),
            operator: assignment_node.operator,
            value: self.exprs.alloc_item(checked_rhs),
            type_id: lhs_ty,
        }));
    }

    fn visit_variable(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let variable_node = ctx.statement(node).as_variable().cloned().unwrap();
        let checked_expr = self.visit_expr(variable_node.value, ctx)?;
        let ty = checked_expr.ty();
        if ty != self.typecheck(&variable_node.ty, ctx)? {
            return Err(Error::TypeMismatch);
        }
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        ctx.symbols.declare_variable(
            variable_node.name,
            CheckedVariable::new(
                ty,
                variable_node.mutable,
                variable_node.cnst,
                current_scope_id,
                None,
            ),
        );
        let checked_variable = CheckedVariableNode {
            name: variable_node.name,
            ty,
            mutable: variable_node.mutable,
            cnst: variable_node.cnst,
            value: self.exprs.alloc_item(checked_expr),
            scope_id: current_scope_id,
        };
        Ok(CheckedStmtNode::Variable(checked_variable))
    }

    fn visit_storage_write(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let storage_node = ctx.statement(node).as_storage().cloned().unwrap();
        let offset = self.visit_expr(storage_node.offset, ctx)?;
        let value = self.visit_expr(storage_node.value, ctx)?;
        if offset.ty() != FELT_TYPE || value.ty() != FELT_TYPE {
            return Err(Error::TypeMismatch);
        }
        Ok(CheckedStmtNode::Storage(CheckedStorageWriteNode {
            offset: self.exprs.alloc_item(offset),
            value: self.exprs.alloc_item(value),
        }))
    }

    fn visit_return(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let return_node = ctx.statement(node).as_return().cloned().unwrap();
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        let parent_scope_id = ctx.symbols.parent_scope_id().unwrap();
        if ctx.symbols[current_scope_id].kind != ScopeKind::Block {
            return Err(Error::InvalidReturn);
        }
        let valid_kinds = [
            ScopeKind::Function,
            ScopeKind::ImplMethod,
            ScopeKind::TraitMethod,
        ];
        if !valid_kinds.contains(&ctx.symbols[parent_scope_id].kind) {
            return Err(Error::InvalidReturn);
        }

        let ret = if let Some(expr) = return_node.0 {
            let expr = self.visit_expr(expr, ctx)?;
            let ty = expr.ty();
            Some((self.exprs.alloc_item(expr), ty))
        } else {
            None
        };

        Ok(CheckedStmtNode::Return(CheckedReturnNode { ret }))
    }

    fn visit_impl(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let impl_node = ctx.definition(node).as_impl().cloned().unwrap();
        if impl_node.trait_name.is_some() {
            return Ok(CheckedDefinitionNode::Impl(
                self.typecheck_impl_trait(&impl_node, ctx)?,
            ));
        }

        let (implementor_scope, type_id) = ctx.symbols.resolve_implementor(impl_node.ty)?;
        ctx.symbols.push_scope(implementor_scope);
        ctx.symbols.start_scope(ScopeKind::Impl);

        ctx.symbols.add_type_id(None, IdentId::TYPE_SELF, type_id);
        ctx.symbols.add_type_id(None, IdentId::SELF, type_id);

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for &generic_parameter in &impl_node.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for &function_id in &impl_node.body {
            methods.push(self.typecheck_method(function_id, ctx)?);
        }
        let checked_impl = CheckedImplNode {
            generic_parameters,
            trait_name: impl_node.trait_name,
            ty: impl_node.ty,
            body: methods,
            scope_id: ctx.symbols.current_scope_id().unwrap(),
        };
        // let ty = Type::Impl(checked_impl.clone());
        // symbols.add_type(symbols.parent_scope_id(), ty);

        ctx.symbols.end_scope();
        ctx.symbols.pop_scope();
        Ok(CheckedDefinitionNode::Impl(checked_impl))
    }

    fn visit_trait(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Trait);
        // TODO: remove clone
        let trait_node = ctx.definition(node).as_trait().cloned().unwrap();

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for &generic_parameter in &trait_node.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for &function_id in &trait_node.body {
            methods.push(self.typecheck_trait_method(function_id, ctx)?);
        }
        let checked_trait = CheckedTraitNode {
            generic_parameters,
            name: trait_node.name,
            body: methods,
            implementors: Vec::new(),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            visibility: trait_node.visibility,
        };
        // TODO: remove clone
        let ty = Type::Trait(checked_trait.clone());
        ctx.symbols.add_type(ctx.symbols.parent_scope_id(), ty);

        ctx.symbols.end_scope();
        Ok(CheckedDefinitionNode::Trait(checked_trait))
    }

    fn visit_function(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let function = ctx.definition(node).as_function().cloned().unwrap();

        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        let mut generic_parameters = Vec::new();
        let mut parameters = Vec::new();

        for &generic_parameter in &function.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for (parameter, mutable, parameter_type) in &function.parameters {
            let parameter_type = self.typecheck(parameter_type, ctx)?;
            let variable =
                CheckedVariable::new(parameter_type, *mutable, false, current_scope_id, None);
            ctx.symbols.declare_variable(parameter.clone(), variable);
            parameters.push((parameter.clone(), *mutable, parameter_type));
        }

        let expected_return_type = if let Some(ref ret) = function.return_type {
            Some(self.typecheck(ret, ctx)?)
        } else {
            None
        };

        let (checked_body, actual_return_type) = if let Some(body) = &function.body {
            let checked_body = self.visit_block(body.clone(), ctx)?;

            let actual_return_type =
                checked_body
                    .as_block()
                    .unwrap()
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
            body: checked_body.map(|x| self.stmts.alloc_item(x)),
            return_type: expected_return_type,
            scope_id: current_scope_id,
            visibility: function.visibility,
            attrs: function.attrs,
        };

        Ok(CheckedDefinitionNode::Function(checked_function))
    }

    fn visit_struct(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Struct);
        // TODO: remove clone
        let struct_node = ctx.definition(node).as_struct().cloned().unwrap();

        let mut generic_parameters = Vec::new();

        for &parameter in &struct_node.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(parameter);
            generic_parameters.push(type_id);
        }

        let mut checked_struct = CheckedStructNode {
            name: struct_node.name.clone(),
            generic_parameters,
            fields: IndexMap::new(),
            implementations: Vec::new(),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            visibility: struct_node.visibility,
        };

        for (field_name, (field_type, visibility)) in &struct_node.fields {
            let field_type = self.typecheck(field_type, ctx)?;
            checked_struct
                .fields
                .insert(field_name.clone(), (field_type, *visibility));

            ctx.symbols
                .add_type_id(None, field_name.clone(), field_type);
        }

        let ty = Type::Struct(checked_struct.clone());
        ctx.symbols.add_type(ctx.symbols.parent_scope_id(), ty);

        ctx.symbols.end_scope();
        Ok(CheckedDefinitionNode::Struct(checked_struct))
    }

    fn visit_enum(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Enum);
        // TODO: remove clone
        let enum_node = ctx.definition(node).as_enum().cloned().unwrap();

        let mut generic_parameters = Vec::new();

        for &generic_parameter in &enum_node.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for variant in &enum_node.variants {
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
            visibility: enum_node.visibility,
        };
        let ty = Type::Enum(checked_enum.clone());
        ctx.symbols.add_type(ctx.symbols.parent_scope_id(), ty);

        ctx.symbols.end_scope();
        Ok(CheckedDefinitionNode::Enum(checked_enum))
    }

    fn visit_expr(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        ctx.push_node_id(NodeId::from(expr_id));
        let res = match ctx.expression(expr_id).node_type() {
            NodeType::PathExpr => self.visit_path(expr_id, ctx)?,
            NodeType::ValueExpr => self.visit_value(expr_id, ctx)?,
            NodeType::BinaryExpr => self.visit_binary(expr_id, ctx)?,
            NodeType::UnaryExpr => self.visit_unary(expr_id, ctx)?,
            NodeType::CallExpr => self.visit_call(expr_id, ctx)?,
            NodeType::CastExpr => self.visit_cast(expr_id, ctx)?,
            NodeType::IndexAccessExpr => self.visit_index_access(expr_id, ctx)?,
            NodeType::MemberAccessExpr => self.visit_member_access(expr_id, ctx)?,
            NodeType::StorageExpr => self.visit_storage_read(expr_id, ctx)?,
            NodeType::ContextExpr => self.visit_context(expr_id, ctx)?,
            NodeType::AssertExpr => self.visit_assert(expr_id, ctx)?,
            NodeType::AssertEqExpr => self.visit_assert_eq(expr_id, ctx)?,
            _ => std::unreachable!(),
        };
        ctx.pop_node_id();
        Ok(res)
    }

    fn visit_definition(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        ctx.push_node_id(NodeId::from(def_id));
        let res = match ctx.definition(def_id).node_type() {
            NodeType::FunctionDef => {
                ctx.symbols.start_scope(ScopeKind::Function);
                let checked_function = self.visit_function(def_id, ctx)?;
                let ty = Type::Function(checked_function.as_function().cloned().unwrap());
                ctx.symbols.add_type(ctx.symbols.parent_scope_id(), ty);

                ctx.symbols.end_scope();

                checked_function
            }
            NodeType::StructDef => self.visit_struct(def_id, ctx)?,
            NodeType::EnumDef => self.visit_enum(def_id, ctx)?,
            NodeType::ImplDef => self.visit_impl(def_id, ctx)?,
            NodeType::TraitDef => self.visit_trait(def_id, ctx)?,
            _ => std::unreachable!(),
        };
        ctx.pop_node_id();
        Ok(res)
    }

    fn visit_stmt(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        ctx.push_node_id(NodeId::from(stmt_id));
        let res = match ctx.statement(stmt_id).node_type() {
            NodeType::IfStmt => self.visit_if(stmt_id, ctx)?,
            NodeType::WhileStmt => self.visit_while(stmt_id, ctx)?,
            NodeType::BlockStmt => self.visit_block(stmt_id, ctx)?,
            NodeType::AssignmentStmt => self.visit_assignment(stmt_id, ctx)?,
            NodeType::VariableStmt => self.visit_variable(stmt_id, ctx)?,
            NodeType::ReturnStmt => self.visit_return(stmt_id, ctx)?,
            NodeType::DefinitionStmt => Self::StmtResult::from({
                let definition = self.visit_definition(
                    ctx.statement(stmt_id).as_definition().unwrap().clone(),
                    ctx,
                )?;
                self.defs.alloc_item(definition)
            }),
            NodeType::ExpressionStmt => Self::StmtResult::from({
                let expr =
                    self.visit_expr(ctx.statement(stmt_id).as_expression().unwrap().clone(), ctx)?;
                self.exprs.alloc_item(expr)
            }),
            NodeType::StorageStmt => self.visit_storage_write(stmt_id, ctx)?,
            _ => std::unreachable!(),
        };
        ctx.pop_node_id();
        Ok(res)
    }

    fn visit_module(
        &mut self,
        module_id: ModuleId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<(), Self::Error> {
        ctx.push_node_id(NodeId::from(module_id));
        // TODO: remove clone
        let module = ctx.module(module_id).clone();
        if module.is_std && module.is_self_prelude {
            self.typecheck_std_prelude_module(&module, ctx)?;
        }

        for use_path in &module.uses {
            self.visit_use(use_path, ctx)?;
        }

        for &def_id in &module.definitions {
            self.visit_definition(def_id, ctx)?;
        }
        ctx.pop_node_id();

        Ok(())
    }

    fn visit_program(&mut self, ctx: &mut Self::Context) -> std::result::Result<(), Self::Error> {
        // TODO: remove clone
        ctx.symbols
            .load_modules(ctx.program().modules.clone().iter());
        let mut colors = HashMap::new();
        ctx.dependency_graph()
            .ts(&ModuleId::root(), &mut colors, &mut |&module_id| {
                ctx.symbols.push_module(module_id);
                self.visit_module(module_id, ctx).unwrap();
                ctx.symbols.pop_module();
            });

        Ok(())
    }
}

impl<F: Clone + From<u32>, C> Index<ExprId> for TypeChecker<F, C> {
    type Output = CheckedExprNode<F>;

    fn index(&self, index: ExprId) -> &Self::Output {
        &self.exprs[index]
    }
}

impl<F: Clone + From<u32>, C> Index<StmtId> for TypeChecker<F, C> {
    type Output = CheckedStmtNode;

    fn index(&self, index: StmtId) -> &Self::Output {
        &self.stmts[index]
    }
}

impl<F: Clone + From<u32>, C> Index<DefId> for TypeChecker<F, C> {
    type Output = CheckedDefinitionNode;

    fn index(&self, index: DefId) -> &Self::Output {
        &self.defs[index]
    }
}

impl<F: Clone + From<u32>, C> TypeChecker<F, C> {
    pub fn new() -> Self {
        Self {
            exprs: Arena::new(),
            stmts: Arena::new(),
            defs: Arena::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn typecheck_std_prelude_module(
        &mut self,
        module: &ModuleNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        STD_PRELUDE_SCOPE_ID.set(ctx.symbols.current_scope_id().unwrap());
        for (ident, ty) in TYPE_MAPPING {
            ctx.symbols.add_type(None, ty.clone());
        }
        Ok(())
    }

    pub fn typecheck_member_access(
        &mut self,
        receiver: ExprId,
        ctx: &TypeCheckerVisitorContext<F, C>,
    ) -> bool {
        ctx.expression(receiver)
            .as_path()
            .map(|x| x.is_receiver())
            .unwrap_or(false)
            && ctx
                .symbols
                .find_scope(None, vec![ScopeKind::Impl], |s| {
                    s.kind == ScopeKind::ImplMethod
                })
                .is_some()
    }

    pub fn typecheck(
        &mut self,
        ty: &UncheckedType,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        match ty {
            UncheckedType::Basic(IdentId::TYPE_BOOL) => Ok(BOOL_TYPE),
            UncheckedType::Basic(IdentId::TYPE_FELT) => Ok(FELT_TYPE),
            UncheckedType::Basic(name) => Ok(ctx
                .symbols
                .get_type_id(None, name.clone())
                .ok_or(Error::UnresolvedType)?),
            UncheckedType::Generic(name, generic_parameters) => {
                let type_id = ctx
                    .symbols
                    .get_type_id(None, name.clone())
                    .ok_or(Error::UnresolvedType)?;

                let mut checked_generic_parameters = Vec::new();
                for generic_parameter in generic_parameters {
                    checked_generic_parameters.push(self.typecheck(generic_parameter, ctx));
                }

                let ty = &ctx.symbols[type_id];
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
                let inner_ty = self.typecheck(inner, ctx)?;
                let scope_id = ScopeId::prelude();
                let type_array = Type::Array(CheckedArrayNode {
                    inner_ty,
                    size: size.clone(),
                    scope_id,
                });
                let type_id = match ctx.symbols.get_type_id(Some(scope_id), type_array.key()){
                    Some(type_id) => type_id,
                    None => {
                        ctx.symbols.add_type(Some(scope_id), type_array)
                    }
                };
                Ok(type_id)
            }
            UncheckedType::Unknown => Ok(UNKOWN_TYPE),
        }
    }

    fn typecheck_trait_method(
        &mut self,
        function_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        ctx.symbols.start_scope(ScopeKind::TraitMethod);
        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, UNKOWN_TYPE);

        let checked_function = self.typecheck_function(function_id, ctx)?;
        let ty = Type::Function(checked_function.clone());
        ctx.symbols.add_type(None, ty);

        ctx.symbols.end_scope();
        Ok(checked_function)
    }

    fn typecheck_method(
        &mut self,
        function_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        ctx.symbols.start_scope(ScopeKind::ImplMethod);
        let checked_function = self.typecheck_function(function_id, ctx)?;
        let ty = Type::Function(checked_function.clone());
        ctx.symbols.add_type(None, ty);

        ctx.symbols.end_scope();
        Ok(checked_function)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_function(
        &mut self,
        function_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        // TODO: remove clone
        let function = ctx.definition(function_id).as_function().cloned().unwrap();
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        let mut generic_parameters = Vec::new();
        let mut parameters = Vec::new();

        for &generic_parameter in &function.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for (parameter, mutable, parameter_type) in &function.parameters {
            let parameter_type = self.typecheck(parameter_type, ctx)?;
            let variable =
                CheckedVariable::new(parameter_type, *mutable, false, current_scope_id, None);
            ctx.symbols.declare_variable(parameter.clone(), variable);
            parameters.push((parameter.clone(), *mutable, parameter_type));
        }

        let expected_return_type = if let Some(ref ret) = function.return_type {
            Some(self.typecheck(ret, ctx)?)
        } else {
            None
        };

        let (checked_body, actual_return_type) = if let Some(body) = &function.body {
            let checked_body = self.visit_block(body.clone(), ctx)?;

            let actual_return_type =
                checked_body
                    .as_block()
                    .unwrap()
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
            body: checked_body.map(|x| self.stmts.alloc_item(x)),
            return_type: expected_return_type,
            scope_id: current_scope_id,
            visibility: function.visibility,
            attrs: function.attrs,
        };

        Ok(checked_function)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_impl_trait(
        &mut self,
        r#impl: &ImplNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedImplNode> {
        let (trait_scope, trait_type_id) = ctx.symbols.resolve_trait(r#impl.trait_name.unwrap())?;
        let (implementor_scope, implementor_type_id) =
            ctx.symbols.resolve_implementor(r#impl.ty)?;
        ctx.symbols.push_scope(trait_scope);
        ctx.symbols.start_scope(ScopeKind::Impl);

        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, implementor_type_id);
        ctx.symbols.add_type_id(
            None,
            ctx.symbols[implementor_type_id].key(),
            implementor_type_id,
        );
        ctx.symbols
            .add_type_id(None, IdentId::SELF, implementor_type_id);

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for &generic_parameter in &r#impl.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter);
            generic_parameters.push(type_id);
        }

        for &function_id in &r#impl.body {
            methods.push(self.typecheck_method(function_id, ctx)?);
        }
        let checked_impl = CheckedImplNode {
            generic_parameters,
            trait_name: r#impl.trait_name,
            ty: r#impl.ty,
            body: methods,
            scope_id: ctx.symbols.current_scope_id().unwrap(),
        };
        let ty = Type::Impl(checked_impl.clone());
        // symbols.add_type(symbols.parent_scope_id(), ty);

        ctx.symbols
            .impl_trait_for_type(trait_type_id, implementor_type_id);

        ctx.symbols.end_scope();
        ctx.symbols.pop_scope();
        Ok(checked_impl)
    }
}
