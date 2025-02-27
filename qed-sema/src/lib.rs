mod definition;
mod expr;
mod stmt;
mod symbol_table;
mod r#type;
mod value;
mod variable;

mod error;

pub use definition::*;
pub use error::*;
pub use expr::*;
use indexmap::IndexMap;
use qed_ast::*;
use qed_common::{Arena, Graph};
pub use r#type::*;
use std::{collections::HashMap, ops::Index};
pub use stmt::*;
pub use symbol_table::*;
pub use value::*;
pub use variable::*;

use qed_ast::BlockExprNode;
// use tracing::{debug, error, info, instrument, span, Level};
use tracing::instrument;

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

    fn intern<S: Into<Ident>>(&mut self, _s: S) -> IdentId {
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

    fn insert_definition(&mut self, _definition: Self::Definition, _pos: InsertPosition) {
        unimplemented!()
    }

    fn alloc_expression(&mut self, _expr: Self::Expr) -> ExprId {
        unimplemented!()
    }

    fn alloc_statement(&mut self, _stmt: Self::Stmt) -> StmtId {
        unimplemented!()
    }

    fn alloc_definition(&mut self, _definition: Self::Definition) -> DefId {
        unimplemented!()
    }

    fn replace_definition(&mut self, _def_id: DefId, _definition: Self::Definition) {
        unimplemented!()
    }

    fn replace_statement(&mut self, _stmt_id: StmtId, _statement: Self::Stmt) {
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
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        ctx.symbols.add_use(use_path)?;
        Ok(CheckedStmtNode::Use)
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

    fn visit_tuple_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // get TupleAccessNode
        let tuple_access_node = ctx.expression(node).as_tuple_access().cloned().unwrap();

        let checked_expr = self.visit_expr(tuple_access_node.target, ctx)?;
        let type_id = checked_expr.ty();
        let ty = &ctx.symbols[type_id];
        let element_types = ty.as_tuple().ok_or(Error::TypeMismatch)?;

        if tuple_access_node.index >= element_types.len() {
            return Err(Error::IndexOutOfBounds);
        }

        let field_type = element_types
            .get(tuple_access_node.index)
            .ok_or(Error::IndexOutOfBounds)?
            .clone();
        Ok(CheckedExprNode::TupleAccess(CheckedTupleAccessNode {
            value: self.exprs.alloc_item(checked_expr),
            index: tuple_access_node.index,
            type_id: field_type,
        }))
    }
    fn visit_intrinsic_expr(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let intrinsic_node = ctx.expression(node).as_intrinsic().cloned().unwrap();
        match intrinsic_node {
            IntrinsicExprNode::GetUserId => {
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetUserId { type_id: FELT_TYPE },
                ));
            }
            IntrinsicExprNode::GetContractId => {
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetContractId { type_id: FELT_TYPE },
                ));
            }
            IntrinsicExprNode::GetCheckpointId => {
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetCheckpointId { type_id: FELT_TYPE },
                ));
            }
            IntrinsicExprNode::GetLastNonce => {
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetLastNonce { type_id: FELT_TYPE },
                ));
            }
            IntrinsicExprNode::GetUserPublicKeyHash => {
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetUserPublicKeyHash { type_id: HASH_TYPE },
                ));
            }
            IntrinsicExprNode::GetStateHashAt { slot_index } => {
                let slot_index = self.visit_expr(slot_index, ctx)?;
                if !self.unify(slot_index.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch);
                }
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetStateHashAt {
                        slot_index: self.exprs.alloc_item(slot_index),
                        type_id: HASH_TYPE,
                    },
                ));
            }
            IntrinsicExprNode::GetOtherContractStateHashAt {
                contract_state_tree_height,
                contract_id,
                slot_index,
            } => {
                let contract_state_tree_height =
                    self.visit_expr(contract_state_tree_height, ctx)?;
                let contract_id = self.visit_expr(contract_id, ctx)?;
                let slot_index = self.visit_expr(slot_index, ctx)?;
                if !self.unify(contract_state_tree_height.ty(), FELT_TYPE, ctx)
                    || !self.unify(contract_id.ty(), FELT_TYPE, ctx)
                    || !self.unify(slot_index.ty(), FELT_TYPE, ctx)
                {
                    return Err(Error::TypeMismatch);
                }

                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetOtherContractStateHashAt {
                        contract_state_tree_height: self
                            .exprs
                            .alloc_item(contract_state_tree_height),
                        contract_id: self.exprs.alloc_item(contract_id),
                        slot_index: self.exprs.alloc_item(slot_index),
                        type_id: HASH_TYPE,
                    },
                ));
            }
            IntrinsicExprNode::GetOtherUserContractStateHashAt {
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
                if !self.unify(contract_state_tree_height.ty(), FELT_TYPE, ctx)
                    || !self.unify(user_id.ty(), FELT_TYPE, ctx)
                    || !self.unify(contract_id.ty(), FELT_TYPE, ctx)
                    || !self.unify(slot_index.ty(), FELT_TYPE, ctx)
                {
                    return Err(Error::TypeMismatch);
                }

                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetOtherUserContractStateHashAt {
                        contract_state_tree_height: self
                            .exprs
                            .alloc_item(contract_state_tree_height),
                        user_id: self.exprs.alloc_item(user_id),
                        contract_id: self.exprs.alloc_item(contract_id),
                        slot_index: self.exprs.alloc_item(slot_index),
                        type_id: HASH_TYPE,
                    },
                ));
            }
            IntrinsicExprNode::CSetStateHashAt {
                slot_index,
                new_value,
            } => {
                let slot_index = self.visit_expr(slot_index, ctx)?;
                let new_value = self.visit_expr(new_value, ctx)?;

                if !self.unify(slot_index.ty(), FELT_TYPE, ctx)
                    || !self.unify(new_value.ty(), HASH_TYPE, ctx)
                {
                    return Err(Error::TypeMismatch);
                }

                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::CSetStateHashAt {
                        slot_index: self.exprs.alloc_item(slot_index),
                        new_value: self.exprs.alloc_item(new_value),
                        type_id: HASH_TYPE,
                    },
                ));
            }
            IntrinsicExprNode::Read { offset } => {
                // TODO: remove clone
                let offset = self.visit_expr(offset, ctx)?;
                if !self.unify(offset.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch);
                }
                return Ok(CheckedExprNode::Intrinsic(CheckedIntrinsicExprNode::Read {
                    offset: self.exprs.alloc_item(offset),
                    type_id: FELT_TYPE,
                }));
            }
            IntrinsicExprNode::Write { offset, value } => {
                // TODO: remove clone
                let offset = self.visit_expr(offset, ctx)?;
                let value = self.visit_expr(value, ctx)?;
                if !self.unify(offset.ty(), FELT_TYPE, ctx)
                    || !self.unify(value.ty(), FELT_TYPE, ctx)
                {
                    return Err(Error::TypeMismatch);
                }
                Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::Write {
                        offset: self.exprs.alloc_item(offset),
                        value: self.exprs.alloc_item(value),
                        type_id: FELT_TYPE,
                    },
                ))
            }
            IntrinsicExprNode::Hash { data } => {
                let data = self.visit_expr(data, ctx)?;

                Ok(CheckedExprNode::Intrinsic(CheckedIntrinsicExprNode::Hash {
                    data: self.exprs.alloc_item(data),
                    type_id: HASH_TYPE,
                }))
            }
        }
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
                        if !self.unify(checked_expr.ty(), inner_ty, ctx) {
                            return Err(Error::TypeMismatch);
                        }
                    } else {
                        inner_ty = Some(checked_expr.ty());
                    }
                    elements.push(self.exprs.alloc_item(checked_expr));
                }

                let scope_id = ScopeId::primitive();
                let type_array = Type::Array(CheckedArrayNode {
                    inner_ty: inner_ty.unwrap(),
                    size: size.clone(),
                });
                let type_id = ctx
                    .symbols
                    .get_type_id(Some(scope_id), type_array.key())
                    .unwrap();

                Ok(CheckedExprNode::Value(CheckedValueNode::Array(
                    type_id, elements,
                )))
            }
            ValueNode::Struct(name, _, data) => Ok({
                let type_id = ctx
                    .symbols
                    .get_type_id(None, name)
                    .ok_or(Error::UnresolvedType)?;
                let mut new_data = IndexMap::new();
                if ctx.symbols[type_id].as_struct().unwrap().fields.len() != data.len() {
                    return Err(Error::TypeMismatch);
                }
                for (k, v) in data.iter() {
                    let expr = self.visit_expr(v.clone(), ctx)?;
                    let t = expr.ty();
                    new_data.insert(k.clone(), self.exprs.alloc_item(expr));

                    let checked_struct = ctx.symbols[type_id].as_struct().unwrap();

                    let (field_name, (field_type, _)) = checked_struct
                        .fields
                        .iter()
                        .find(|(field_name, _)| *field_name == k)
                        .unwrap();

                    if k != field_name || !self.unify(t, *field_type, ctx) {
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
        if !self.unify(lhs_ty, checked_rhs.ty(), ctx) {
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
                if !self.unify(lhs_ty, BOOL_TYPE, ctx) {
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

        if !self.unify(type_id, FELT_TYPE, ctx) && !self.unify(type_id, BOOL_TYPE, ctx) {
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
        let callee = self.visit_expr(call_node.callee, ctx)?;
        let ty = callee.ty();
        // TODO: remove clone
        let f = ctx.symbols[ty].clone();
        let (args, generic_parameters, return_type) = match f {
            Type::Function(n) => {
                if call_node.args.len() != n.parameters.len() {
                    return Err(Error::InvalidFunctionCall);
                }
                let mut args = Vec::new();
                for (i, arg) in call_node.args.iter().enumerate() {
                    let type_arg = self.visit_expr(arg.clone(), ctx)?;
                    if !self.unify(type_arg.ty(), n.parameters[i].2, ctx) {
                        return Err(Error::FunctionParameterMismatch);
                    }
                    args.push(type_arg);
                }
                (args, n.generic_parameters.clone(), n.return_type)
            }
            Type::FunctionSignature(sig) => {
                if call_node.args.len() != sig.parameters.len() {
                    return Err(Error::InvalidFunctionCall);
                }
                let mut args = Vec::new();
                for (i, arg) in call_node.args.iter().enumerate() {
                    let type_arg = self.visit_expr(arg.clone(), ctx)?;
                    if !self.unify(type_arg.ty(), sig.parameters[i].1, ctx) {
                        return Err(Error::FunctionParameterMismatch);
                    }
                    args.push(type_arg);
                }
                (args, vec![], sig.return_type)
            }
            _ => return Err(Error::TypeMismatch),
        };

        return Ok(CheckedExprNode::Call(CheckedCallNode {
            callee: self.exprs.alloc_item(callee),
            generic_parameters: generic_parameters.clone(),
            args: self.exprs.alloc_items(args),
            type_id: return_type,
        }));
    }

    fn visit_member_call(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let call_node = ctx.expression(node).as_member_call().cloned().unwrap();
        let variable = self.visit_expr(call_node.callee, ctx)?;
        let ty = variable.ty();
        // TODO: remove clone
        let f = ctx.symbols[ty].as_function().unwrap().clone();
        let mut args = Vec::new();
        // TODO: add member call
        let receiver = {
            let expr = self.visit_expr(call_node.receiver, ctx)?;
            if !self.unify(expr.ty(), f.parameters[0].2, ctx) {
                return Err(Error::FunctionParameterMismatch);
            }
            self.exprs.alloc_item(expr)
        };

        for (i, arg) in call_node.args.iter().enumerate() {
            let type_arg = self.visit_expr(arg.clone(), ctx)?;
            if !self.unify(type_arg.ty(), f.parameters[i + 1].2, ctx) {
                return Err(Error::FunctionParameterMismatch);
            }
            args.push(type_arg);
        }

        return Ok(CheckedExprNode::MemberCall(CheckedMemberCallNode {
            callee: self.exprs.alloc_item(variable),
            receiver,
            generic_parameters: f.generic_parameters.clone(),
            args: self.exprs.alloc_items(args),
            type_id: f.return_type,
        }));
    }

    fn visit_tuple(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        let tuple_node = ctx.expression(node).as_tuple().cloned().unwrap();

        let checked_elements: Result<Vec<CheckedExprNode<F>>> = tuple_node
            .elements
            .iter()
            .map(|expr_id| self.visit_expr(*expr_id, ctx))
            .collect();

        let checked_elements = checked_elements?;
        let element_types: Vec<TypeId> = checked_elements.iter().map(|e| e.ty()).collect();

        let tuple_type = Type::Tuple(element_types.clone());
        let scope_id = ScopeId::primitive();
        let type_id = if let Some(tid) = ctx.symbols.get_type_id(Some(scope_id), tuple_type.key()) {
            tid
        } else {
            ctx.symbols.add_type(Some(scope_id), tuple_type)?
        };

        let elements_with_types = checked_elements
            .into_iter()
            .map(|e| (e.ty(), self.exprs.alloc_item(e)))
            .collect();

        let checked_expr =
            CheckedExprNode::Value(CheckedValueNode::Tuple(type_id, elements_with_types));

        Ok(checked_expr)
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

        if !self.unify(src_type, FELT_TYPE, ctx) && !self.unify(target_type, BOOL_TYPE, ctx)
            || !self.unify(src_type, BOOL_TYPE, ctx) && !self.unify(target_type, FELT_TYPE, ctx)
        {
            return Ok(CheckedExprNode::Cast(CheckedCastNode {
                value: self.exprs.alloc_item(src_expr),
                target_type,
            }));
        } else {
            return Err(Error::TypeMismatch);
        };
    }

    fn visit_if_expr(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let if_expr_node = ctx.expression(node).as_if_expr().cloned().unwrap();
        let checked_expr = self.visit_expr(if_expr_node.if_branch.predicate, ctx)?;
        // TODO: optimize the check
        if checked_expr.ty() != BOOL_TYPE {
            println!(
                "Error: type mismatch, expected Bool, but found {:?} ",
                checked_expr.ty()
            );
            return Err(Error::TypeMismatch);
        }

        let checked_block = self.visit_stmt(if_expr_node.if_branch.body, ctx)?;

        let if_type = match checked_block {
            //actually, it should be a block
            CheckedStmtNode::Expression(expr) => Some(self.exprs[expr].ty()),
            _ => None,
        };
        let if_branch = CheckedCase {
            predicate: self.exprs.alloc_item(checked_expr).clone(),
            type_id: BOOL_TYPE,
            body: self.stmts.alloc_item(checked_block),
        };

        let mut elseif_branches = Vec::with_capacity(if_expr_node.elseif_branches.len());
        for branch in &if_expr_node.elseif_branches {
            let checked_expr = self.visit_expr(branch.predicate, ctx)?;
            if checked_expr.ty() != BOOL_TYPE {
                println!(
                    "Error: type mismatch, expected Bool, but found {:?} ",
                    checked_expr.ty()
                );
                return Err(Error::TypeMismatch);
            }
            let checked_block = self.visit_stmt(branch.body, ctx)?;
            // check_block_type(&checked_block, if_type)?;
            let checked_block_type = match checked_block {
                CheckedStmtNode::Expression(expr) => Some(self.exprs[expr].ty()),
                _ => None,
            };
            if checked_block_type != if_type {
                println!(
                    "Error: type mismatch, expected {:?}, but found {:?} ",
                    if_type, checked_block_type
                );
                return Err(Error::TypeMismatch);
            }

            elseif_branches.push(CheckedCase {
                predicate: self.exprs.alloc_item(checked_expr).clone(),
                type_id: BOOL_TYPE,
                body: self.stmts.alloc_item(checked_block),
            });
        }

        let else_branch = if let Some(else_branch) = if_expr_node.else_branch {
            let block = self.visit_stmt(else_branch, ctx)?;
            // check_block_type(&block, if_type)?;
            let checked_block_type = match block {
                CheckedStmtNode::Expression(expr) => Some(self.exprs[expr].ty()),
                _ => None,
            };
            if checked_block_type != if_type {
                println!(
                    "Error: type mismatch, expected {:?}, but found {:?} ",
                    if_type, checked_block_type
                );
                return Err(Error::TypeMismatch);
            }

            Some(self.stmts.alloc_item(block))
        } else {
            None
        };

        Ok(CheckedExprNode::IfExpr(CheckedIfExprNode {
            if_branch,
            elseif_branches,
            else_branch,
            type_id: TypeId::from(if_type.unwrap_or(VOID_TYPE)),
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
        if !self.unify(predicate.ty(), BOOL_TYPE, ctx) {
            return Err(Error::TypeMismatch);
        }
        let checked_block = self.visit_stmt(while_node.body, ctx)?;
        Ok(CheckedStmtNode::While(CheckedWhileNode {
            predicate: self.exprs.alloc_item(predicate),
            type_id: BOOL_TYPE,
            body: self.stmts.alloc_item(checked_block),
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

        if !self.unify(lhs_ty, checked_rhs.ty(), ctx) {
            return Err(Error::TypeMismatch);
        }

        Ok(CheckedStmtNode::Assignment(CheckedAssignmentNode {
            variable: self.exprs.alloc_item(checked_lhs),
            operator: assignment_node.operator,
            value: self.exprs.alloc_item(checked_rhs),
            type_id: lhs_ty,
        }))
    }

    fn visit_variable(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let variable_node = ctx.statement(node).as_variable().cloned().unwrap();
        let lhs_ty = self.typecheck(&variable_node.ty, ctx)?;
        let checked_expr = self.visit_expr(variable_node.value, ctx)?;
        let rhs_ty = checked_expr.ty();
        if !self.unify(rhs_ty, lhs_ty, ctx) {
            return Err(Error::TypeMismatch);
        }
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        ctx.symbols.declare_variable(
            variable_node.name,
            CheckedVariable::new(rhs_ty, variable_node.mutable, current_scope_id, None),
        )?;
        let checked_variable = CheckedVariableNode {
            name: variable_node.name,
            ty: rhs_ty,
            mutable: variable_node.mutable,
            cnst: variable_node.cnst,
            value: self.exprs.alloc_item(checked_expr),
            scope_id: current_scope_id,
        };
        Ok(CheckedStmtNode::Variable(checked_variable))
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
            //todo!: check this
            ScopeKind::Block,
        ];
        if !valid_kinds.contains(&ctx.symbols[parent_scope_id].kind) {
            return Err(Error::InvalidReturn);
        }

        let ret = if let Some(expr) = return_node.0 {
            let expr = self.visit_expr(expr, ctx)?;
            Some(self.exprs.alloc_item(expr))
        } else {
            None
        };

        Ok(CheckedStmtNode::Return(CheckedReturnNode { ret }))
    }

    fn visit_intrinsic_stmt(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        let node = ctx.statement(node).as_intrinsic().cloned().unwrap();
        match node {
            IntrinsicStmtNode::Assert { left, message } => {
                let checked_lhs = self.visit_expr(left, ctx)?;

                if !self.unify(checked_lhs.ty(), BOOL_TYPE, ctx) {
                    return Err(Error::TypeMismatch);
                }

                Ok(CheckedStmtNode::Intrinsic(
                    CheckedIntrinsicStmtNode::Assert {
                        left: self.exprs.alloc_item(checked_lhs),
                        message: message,
                    },
                ))
            }
            IntrinsicStmtNode::AssertEq {
                left,
                right,
                message,
            } => {
                let checked_lhs = self.visit_expr(left, ctx)?;
                let checked_rhs = self.visit_expr(right, ctx)?;

                if !self.unify(checked_lhs.ty(), checked_rhs.ty(), ctx) {
                    return Err(Error::TypeMismatch);
                }

                Ok(CheckedStmtNode::Intrinsic(
                    CheckedIntrinsicStmtNode::AssertEq {
                        left: self.exprs.alloc_item(checked_lhs),
                        right: self.exprs.alloc_item(checked_rhs),
                        message: message,
                    },
                ))
            }
        }
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

        ctx.symbols.add_type_id(None, IdentId::TYPE_SELF, type_id)?;
        ctx.symbols.add_type_id(None, IdentId::SELF, type_id)?;

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for &generic_parameter in &impl_node.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter)?;
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
            let type_id = ctx.symbols.add_type_variable(generic_parameter)?;
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
        ctx.symbols.add_type(ctx.symbols.parent_scope_id(), ty)?;

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
            let type_id = ctx.symbols.add_type_variable(generic_parameter)?;
            generic_parameters.push(type_id);
        }

        let current_scope_kind = &ctx.symbols[current_scope_id].kind;
        let is_method_scope = current_scope_kind == &ScopeKind::ImplMethod
            || current_scope_kind == &ScopeKind::TraitMethod;

        for (i, (parameter, _, parameter_type)) in function.parameters.iter().enumerate() {
            // self parameter is only allowed in associated functions associated functions are those in `impl` or `trait` definitions
            if i > 0 || (i == 0 && !is_method_scope) {
                if parameter == &IdentId::SELF {
                    return Err(Error::InvalidSelfParameter);
                }
            }

            // Self is only available in impls, traits, and type definitions
            if !is_method_scope && parameter_type == &UncheckedType::Basic(IdentId::TYPE_SELF) {
                return Err(Error::InvalidSelfParameter);
            }
        }

        for (parameter, mutable, parameter_type) in &function.parameters {
            let parameter_type = self.typecheck(parameter_type, ctx)?;
            let variable = CheckedVariable::new(parameter_type, *mutable, current_scope_id, None);
            ctx.symbols.declare_variable(parameter.clone(), variable)?;
            parameters.push((parameter.clone(), *mutable, parameter_type));
        }

        let expected_return_type = if let Some(ref ret) = function.return_type {
            self.typecheck(ret, ctx)?
        } else {
            VOID_TYPE
        };

        let checked_body = if let Some(body) = &function.body {
            let checked_body = self.visit_stmt(body.clone(), ctx)?;
            let actual_return_type = match &checked_body {
                CheckedStmtNode::Return(expr) => match expr {
                    CheckedReturnNode { ret: Some(expr) } => self.exprs[expr.clone()].ty(),
                    CheckedReturnNode { ret: None } => VOID_TYPE,
                },
                CheckedStmtNode::Expression(expr) => match self.exprs[expr.clone()].node_type() {
                    NodeType::BlockExpr | NodeType::IfExpr => self.exprs[expr.clone()].ty(),
                    _ => VOID_TYPE,
                },
                _ => VOID_TYPE,
            };
            if expected_return_type != actual_return_type {
                println!(
                    "TypeMismatch expected_type = {:?} , actual_return_type = {:?}",
                    expected_return_type, actual_return_type
                );
                return Err(Error::TypeMismatch);
            }

            Some(checked_body)
        } else {
            None
        };

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
            let type_id = ctx.symbols.add_type_variable(parameter)?;
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
                .add_type_id(None, field_name.clone(), field_type)?;
        }

        let ty = Type::Struct(checked_struct.clone());
        ctx.symbols.add_type(ctx.symbols.parent_scope_id(), ty)?;

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
            let type_id = ctx.symbols.add_type_variable(generic_parameter)?;
            generic_parameters.push(type_id);
        }

        for variant in &enum_node.variants {
            match variant {
                EnumVariant::Basic(_name) => todo!(),
                EnumVariant::Tuple(_name, _members) => todo!(),
                EnumVariant::Struct(_name, _fields) => todo!(),
            }
        }
        todo!();
        // let checked_enum = CheckedEnumNode {
        //     generic_parameters,
        //     name: todo!(),
        //     variants: todo!(),
        //     scope_id: todo!(),
        //     implementations: Vec::new(),
        //     visibility: enum_node.visibility,
        // };
        // let ty = Type::Enum(checked_enum.clone());
        // ctx.symbols.add_type(ctx.symbols.parent_scope_id(), ty)?;
        //
        // ctx.symbols.end_scope();
        // Ok(CheckedDefinitionNode::Enum(checked_enum))
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
            NodeType::MemberCallExpr => self.visit_member_call(expr_id, ctx)?,
            NodeType::CastExpr => self.visit_cast(expr_id, ctx)?,
            NodeType::IndexAccessExpr => self.visit_index_access(expr_id, ctx)?,
            NodeType::MemberAccessExpr => self.visit_member_access(expr_id, ctx)?,
            NodeType::IntrinsicExpr => self.visit_intrinsic_expr(expr_id, ctx)?,
            NodeType::BlockExpr => self.visit_block_expr(expr_id, ctx)?,
            NodeType::IfExpr => self.visit_if_expr(expr_id, ctx)?,
            NodeType::TupleExpr => self.visit_tuple(expr_id, ctx)?,
            NodeType::TupleAccessExpr => self.visit_tuple_access(expr_id, ctx)?,
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
                ctx.symbols.add_type(ctx.symbols.parent_scope_id(), ty)?;

                ctx.symbols.end_scope();

                checked_function
            }
            NodeType::StructDef => self.visit_struct(def_id, ctx)?,
            NodeType::EnumDef => self.visit_enum(def_id, ctx)?,
            NodeType::ImplDef => self.visit_impl(def_id, ctx)?,
            NodeType::TraitDef => self.visit_trait(def_id, ctx)?,
            NodeType::TypeAliasDef => self.visit_type_alias(def_id, ctx)?,
            NodeType::ConstDef => self.visit_const(def_id, ctx)?,
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
            NodeType::WhileStmt => self.visit_while(stmt_id, ctx)?,
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
            NodeType::IntrinsicStmt => self.visit_intrinsic_stmt(stmt_id, ctx)?,
            NodeType::UseStmt => unreachable!(),
            _ => unreachable!(),
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
        if module.is_std && module.is_self_primitive {
            self.typecheck_std_primitive_module(ctx)?;
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
            })
            .unwrap();

        Ok(())
    }

    fn visit_block_expr(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Block);

        let BlockExprNode { stmts, uses } = ctx.expression(node).as_block_expr().unwrap().clone();
        let mut checked_stmts = Vec::with_capacity(stmts.len());

        for &stmt in uses.iter() {
            let use_path = ctx.statement(stmt).as_use().cloned().unwrap();
            self.visit_use(&use_path, ctx)?;
        }

        for stmt in stmts {
            let checked_stmt = self.visit_stmt(stmt, ctx)?;
            checked_stmts.push(checked_stmt);
        }

        //only the last statement in the block can have a return type
        let mut insert_return = None;

        let type_id = match checked_stmts.last() {
            Some(s) => match s {
                CheckedStmtNode::Return(ret) => match ret {
                    CheckedReturnNode { ret: Some(expr) } => self.exprs[expr.clone()].ty(),
                    CheckedReturnNode { ret: None } => VOID_TYPE,
                },
                //when block embedded in a block
                CheckedStmtNode::Expression(expr) => {
                    let type_id = match self.exprs[expr.clone()].node_type() {
                        NodeType::BlockExpr | NodeType::IfExpr => {
                            insert_return = Some(CheckedStmtNode::Return(CheckedReturnNode {
                                ret: Some(expr.clone()),
                            }));
                            self.exprs[expr.clone()].ty()
                        }
                        _ => VOID_TYPE,
                    };
                    type_id
                }
                _ => VOID_TYPE,
            },
            None => VOID_TYPE,
        };
        match insert_return {
            Some(s) => checked_stmts.push(s),
            None => (),
        }
        ctx.symbols.end_scope();

        Ok(CheckedExprNode::BlockExpr(CheckedBlockExprNode {
            stmts: self.stmts.alloc_items(checked_stmts),
            type_id,
        }))
    }

    fn visit_type_alias(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let node = ctx.definition(node).as_type_alias().cloned().unwrap();

        let type_id = self.typecheck(&node.ty, ctx)?;
        ctx.symbols.add_type_id(None, node.name, type_id)?;

        Ok(CheckedDefinitionNode::TypeAlias(CheckedTypeAliasNode {
            name: node.name,
            ty: type_id,
        }))
    }

    fn visit_const(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let node = ctx.definition(node).as_const().cloned().unwrap();
        let type_id = self.typecheck(&node.ty, ctx)?;
        let value = self.visit_expr(node.value, ctx)?;
        if !self.unify(type_id, value.ty(), ctx) {
            return Err(Error::TypeMismatch);
        }

        let node = CheckedConstNode {
            name: node.name,
            ty: type_id,
            value: self.exprs.alloc_item(value),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            visibility: node.visibility,
        };

        ctx.symbols.add_type(None, Type::Const(node.clone()))?;

        Ok(CheckedDefinitionNode::Const(node))
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

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_std_primitive_module(
        &mut self,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        unsafe {
            STD_PRIMITIVE_SCOPE_ID
                .set(ctx.symbols.current_scope_id().unwrap())
                .unwrap()
        };
        for (id, ty) in &*TYPE_MAPPING {
            let key = ty.key();
            let type_id = ctx.symbols.add_type(None, ty.clone())?;
            if id != &key.name {
                ctx.symbols.add_type_id(None, id.clone(), type_id)?;
            }
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
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

    #[instrument(level = "debug", skip_all)]
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
                    Type::Struct(_checked_struct) => {
                        // instantiate struct
                        todo!()
                    }
                    Type::Enum(_checked_enum) => {
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
                let scope_id = ScopeId::primitive();
                let type_array = Type::Array(CheckedArrayNode {
                    inner_ty,
                    size: size.clone(),
                });
                if let Some(type_id) = ctx.symbols.get_type_id(Some(scope_id), type_array.key()) {
                    Ok(type_id)
                } else {
                    Ok(ctx.symbols.add_type(Some(scope_id), type_array)?)
                }
            }
            UncheckedType::Tuple(elements) => {
                // check each element and collect results into a Result<Vec<TypeId>>
                let checked_elements: Result<Vec<TypeId>> = elements
                    .iter()
                    .map(|elem_ty| self.typecheck(elem_ty, ctx))
                    .collect();

                let checked_elements = checked_elements?;

                let checked_tuple = Type::Tuple(checked_elements);

                let scope_id = ScopeId::primitive();
                if let Some(type_id) = ctx.symbols.get_type_id(Some(scope_id), checked_tuple.key())
                {
                    Ok(type_id)
                } else {
                    Ok(ctx.symbols.add_type(Some(scope_id), checked_tuple)?)
                }
            }
            UncheckedType::Unknown => Ok(UNKOWN_TYPE),
            UncheckedType::FunctionSignature(function_signature) => {
                let mut parameters = Vec::with_capacity(function_signature.parameters.len());
                for (mutable, parameter_ty) in &function_signature.parameters {
                    let ty = self.typecheck(parameter_ty, ctx)?;
                    parameters.push((*mutable, ty));
                }
                let return_type = if let Some(ref ty) = function_signature.return_type {
                    Some(self.typecheck(ty, ctx)?)
                } else {
                    None
                };

                let ty = Type::FunctionSignature(CheckedFunctionSignature {
                    parameters,
                    return_type: return_type.unwrap_or(VOID_TYPE),
                });

                if let Some(type_id) = ctx.symbols.get_type_id(None, ty.key()) {
                    Ok(type_id)
                } else {
                    Ok(ctx.symbols.add_type(None, ty)?)
                }
            }
        }
    }

    fn typecheck_trait_method(
        &mut self,
        function_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        ctx.symbols.start_scope(ScopeKind::TraitMethod);
        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, UNKOWN_TYPE)?;

        let checked_function = self.typecheck_function(function_id, ctx)?;
        let ty = Type::Function(checked_function.clone());
        ctx.symbols.add_type(None, ty)?;

        ctx.symbols.end_scope();
        Ok(checked_function)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_method(
        &mut self,
        function_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        ctx.symbols.start_scope(ScopeKind::ImplMethod);
        let checked_function = self.typecheck_function(function_id, ctx)?;
        let ty = Type::Function(checked_function.clone());
        ctx.symbols.add_type(None, ty)?;

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
            let type_id = ctx.symbols.add_type_variable(generic_parameter)?;
            generic_parameters.push(type_id);
        }

        let current_scope_kind = &ctx.symbols[current_scope_id].kind;
        let is_method_scope = current_scope_kind == &ScopeKind::ImplMethod
            || current_scope_kind == &ScopeKind::TraitMethod;

        for (i, (parameter, _, parameter_type)) in function.parameters.iter().enumerate() {
            // self parameter is only allowed in associated functions associated functions are those in `impl` or `trait` definitions
            if i > 0 || (i == 0 && !is_method_scope) {
                if parameter == &IdentId::SELF {
                    return Err(Error::InvalidSelfParameter);
                }
            }

            // Self is only available in impls, traits, and type definitions
            if !is_method_scope && parameter_type == &UncheckedType::Basic(IdentId::TYPE_SELF) {
                return Err(Error::InvalidSelfParameter);
            }
        }

        for (parameter, mutable, parameter_type) in &function.parameters {
            let parameter_type = self.typecheck(parameter_type, ctx)?;
            let variable = CheckedVariable::new(parameter_type, *mutable, current_scope_id, None);
            ctx.symbols.declare_variable(parameter.clone(), variable)?;
            parameters.push((parameter.clone(), *mutable, parameter_type));
        }

        let expected_return_type = if let Some(ref ret) = function.return_type {
            self.typecheck(ret, ctx)?
        } else {
            VOID_TYPE
        };

        let checked_body = if let Some(body) = &function.body {
            let checked_body = self.visit_stmt(body.clone(), ctx)?;
            let actual_return_type = match &checked_body {
                CheckedStmtNode::Return(expr) => match expr {
                    CheckedReturnNode { ret: Some(expr) } => self.exprs[expr.clone()].ty(),
                    CheckedReturnNode { ret: None } => VOID_TYPE,
                },
                CheckedStmtNode::Expression(expr) => match self.exprs[expr.clone()].node_type() {
                    NodeType::BlockExpr | NodeType::IfExpr => self.exprs[expr.clone()].ty(),
                    _ => VOID_TYPE,
                },
                _ => VOID_TYPE,
            };
            if expected_return_type != actual_return_type {
                println!(
                    "expected = {:?}, actual = {:?}",
                    expected_return_type, actual_return_type
                );
                return Err(Error::TypeMismatch);
            }
            Some(checked_body)
        } else {
            None
        };

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
        let (_, implementor_type_id) = ctx.symbols.resolve_implementor(r#impl.ty)?;
        ctx.symbols.push_scope(trait_scope);
        ctx.symbols.start_scope(ScopeKind::Impl);

        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, implementor_type_id)?;
        ctx.symbols.add_type_id(
            None,
            ctx.symbols[implementor_type_id].key(),
            implementor_type_id,
        )?;
        ctx.symbols
            .add_type_id(None, IdentId::SELF, implementor_type_id)?;

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for &generic_parameter in &r#impl.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter)?;
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

        // let ty = Type::Impl(checked_impl.clone());
        // symbols.add_type(symbols.parent_scope_id(), ty);

        ctx.symbols
            .impl_trait_for_type(trait_type_id, implementor_type_id);

        ctx.symbols.end_scope();
        ctx.symbols.pop_scope();
        Ok(checked_impl)
    }

    fn unify(&self, ty1: TypeId, ty2: TypeId, ctx: &TypeCheckerVisitorContext<F, C>) -> bool {
        match (&ctx.symbols[ty1], &ctx.symbols[ty2]) {
            (Type::Function(f), Type::FunctionSignature(sig)) => &f.signature() == sig,
            (Type::FunctionSignature(sig), Type::Function(f)) => &f.signature() == sig,
            (Type::Const(c), Type::Felt(_)) => c.ty == ty2,
            (Type::Felt(_), Type::Const(c)) => c.ty == ty1,
            (Type::Const(c), Type::Const(d)) => c.ty == d.ty,
            _ => ty1 == ty2,
        }
    }
}
