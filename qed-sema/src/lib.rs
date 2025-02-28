mod definition;
mod expr;
mod stmt;
mod symbol_table;
mod traits;
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
pub use stmt::*;
pub use symbol_table::*;
pub use value::*;
pub use variable::*;

use std::{collections::HashMap, ops::Index};
use tracing::instrument;

pub struct TypeCheckerVisitorContext<F: Clone + From<u32>, C> {
    path_stack: Vec<NodeId>,
    pub program: Program<F>,
    pub symbols: SymbolTable<F>,
    inferences: Vec<Vec<HashMap<TypeId, TypeId>>>,
    _marker: std::marker::PhantomData<(F, C)>,
}

impl<F: Clone + From<u32>, C> TypeCheckerVisitorContext<F, C> {
    pub fn new(program: Program<F>) -> Self {
        TypeCheckerVisitorContext {
            path_stack: vec![],
            program,
            symbols: SymbolTable::new(),
            inferences: vec![vec![HashMap::new()]],
            _marker: std::marker::PhantomData,
        }
    }

    pub fn push_inferences_context(&mut self) {
        self.inferences.push(vec![HashMap::new()]);
    }

    pub fn has_inferences(&self) -> bool {
        self.inferences
            .last()
            .unwrap()
            .iter()
            .any(|x| !x.is_empty())
    }

    pub fn resolve_type(&self, type_id: TypeId) -> Option<TypeId> {
        self.inferences
            .last()
            .unwrap()
            .iter()
            .rev()
            .find_map(|x| x.get(&type_id))
            .cloned()
    }

    pub fn add_inference(&mut self, lhs_ty: TypeId, rhs_ty: TypeId) {
        self.inferences
            .last_mut()
            .unwrap()
            .last_mut()
            .unwrap()
            .insert(lhs_ty, rhs_ty);
    }

    pub fn pop_inferences_context(&mut self) {
        self.inferences.pop();
    }

    pub fn push_inferences(&mut self) {
        self.inferences.last_mut().unwrap().push(HashMap::new());
    }

    pub fn pop_inferences(&mut self) {
        self.inferences.last_mut().unwrap().pop();
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

    fn intern_lambda(&mut self) -> IdentId {
        self.program.interner.intern_lambda()
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

    #[instrument(level = "debug", skip_all)]
    fn visit_use(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let node = ctx.definition(def_id).as_use().cloned().unwrap();

        ctx.symbols.add_use(&node)?;
        Ok(CheckedDefinitionNode::Use(node))
    }

    #[instrument(level = "debug", skip_all)]
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
                type_id: self.substitute_all(type_id, ctx)?,
                scope_id,
            }));
        } else {
            return Err(Error::UnresolvedPath);
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_index_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let index_access_node = ctx.expression(node).as_index_access().cloned().unwrap();
        let checked_expr = self.visit_expr(index_access_node.target, ctx)?;
        let checked_index = self.visit_expr(index_access_node.index, ctx)?;

        let type_id = checked_expr.ty();
        let inner_ty = ctx.symbols[type_id].as_array().unwrap().inner_ty.clone();

        if checked_index.ty() != FELT_TYPE {
            return Err(Error::TypeMismatch);
        }

        Ok(CheckedExprNode::IndexAccess(CheckedIndexAccessNode {
            value: self.exprs.alloc_item(checked_expr),
            index: self.exprs.alloc_item(checked_index),
            type_id: self.substitute_all(inner_ty, ctx)?,
        }))
    }

    #[instrument(level = "debug", skip_all)]
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

        let (fields, underlying_type_id) = if let Some(s) = ty.as_struct() {
            (&s.fields, type_id)
        } else {
            let (&underlying_type_id, generic_args) = ty.as_generic_instance().unwrap();

            self.populate_generic_arguments(underlying_type_id, generic_args.to_vec(), ctx)?;

            (
                &ctx.symbols[underlying_type_id].as_struct().unwrap().fields,
                underlying_type_id,
            )
        };

        if let Some((field_type, visibility)) = fields.get(&member_access_node.field) {
            assert!(
                visibility.is_public()
                    || self.typecheck_member_access(member_access_node.target, ctx)
            );
            return Ok(CheckedExprNode::MemberAccess(CheckedMemberAccessNode {
                value: self.exprs.alloc_item(checked_expr),
                field: member_access_node.field.clone(),
                type_id: self.substitute_all(field_type.clone(), ctx)?,
            }));
        }

        let type_id = ctx
            .symbols
            .resolve_method(underlying_type_id, member_access_node.field)
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

    #[instrument(level = "debug", skip_all)]
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
    #[instrument(level = "debug", skip_all)]
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

    #[instrument(level = "debug", skip_all)]
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
            ValueNode::U32(u) => Ok(CheckedExprNode::Value(CheckedValueNode::U32(u.clone()))),
            ValueNode::Array(size, arr) => {
                if size != arr.len() {
                    return Err(Error::TypeMismatch);
                }

                let mut inner_ty = UNKOWN_TYPE;
                let mut elements = Vec::with_capacity(arr.len());
                for e in arr {
                    // TODO: remove clone
                    let checked_expr = self.visit_expr(e, ctx)?;
                    if !self.unify(checked_expr.ty(), inner_ty, ctx) {
                        return Err(Error::TypeMismatch);
                    }
                    inner_ty = checked_expr.ty();
                    elements.push(self.exprs.alloc_item(checked_expr));
                }

                let scope_id = ScopeId::primitive();
                let ty = Type::Array(CheckedArrayNode {
                    inner_ty: self.substitute_all(inner_ty, ctx)?,
                    size: size.clone(),
                });
                let type_id = ctx.symbols.get_or_add_type(Some(scope_id), ty.key(), ty)?;

                Ok(CheckedExprNode::Value(CheckedValueNode::Array(
                    type_id, elements,
                )))
            }
            ValueNode::Struct(name, generic_args, data) => Ok({
                let underlying_type_id = ctx
                    .symbols
                    .get_type_id(None, name)
                    .ok_or(Error::UnresolvedType)?;
                let fields = ctx.symbols[underlying_type_id]
                    .as_struct()
                    .unwrap()
                    .fields
                    .clone();
                let generic_parameters = ctx.symbols[underlying_type_id].generic_parameters();
                if fields.len() != data.len() {
                    return Err(Error::TypeMismatch);
                }

                let mut new_data = IndexMap::new();
                for (field_name, (field_type, _)) in fields {
                    let field_value =
                        self.visit_expr(data.get(&field_name).unwrap().clone(), ctx)?;
                    if !self.unify(field_type, field_value.ty(), ctx) {
                        return Err(Error::TypeMismatch);
                    }
                    new_data.insert(field_name, self.exprs.alloc_item(field_value));
                }

                for (generic_arg, generic_param) in generic_args
                    .clone()
                    .iter()
                    .zip(generic_parameters.clone().into_iter())
                {
                    let generic_arg = self.typecheck(generic_arg, ctx)?;
                    if !self.unify(generic_param, generic_arg, ctx) {
                        return Err(Error::TypeMismatch);
                    }
                }

                let type_id = if generic_parameters.is_empty() {
                    underlying_type_id
                } else {
                    let ty = Type::GenericInstance(underlying_type_id, generic_parameters);

                    let type_id = ctx.symbols.get_or_add_type(None, ty.key(), ty)?;
                    self.substitute_all(type_id, ctx)?
                };

                CheckedExprNode::Value(CheckedValueNode::Struct(type_id, new_data))
            }),
        }
    }

    #[instrument(level = "debug", skip_all)]
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
            | BinaryOperator::Pow
            | BinaryOperator::Mod
            | BinaryOperator::BitShr
            | BinaryOperator::BitShl
            | BinaryOperator::BitAnd
            | BinaryOperator::BitOr
            | BinaryOperator::BitXor => {
                if self.unify(lhs_ty, FELT_TYPE, ctx) {
                    FELT_TYPE
                } else if self.unify(lhs_ty, U32_TYPE, ctx) {
                    U32_TYPE
                } else {
                    return Err(Error::TypeMismatch);
                }
            }
            BinaryOperator::And | BinaryOperator::Or => {
                if !self.unify(lhs_ty, BOOL_TYPE, ctx) {
                    return Err(Error::TypeMismatch);
                }
                BOOL_TYPE
            }
            BinaryOperator::Eq | BinaryOperator::Neq => {
                if !self.unify(lhs_ty, BOOL_TYPE, ctx)
                    && !self.unify(lhs_ty, FELT_TYPE, ctx)
                    && !self.unify(lhs_ty, U32_TYPE, ctx)
                {
                    return Err(Error::TypeMismatch);
                }
                BOOL_TYPE
            }
            BinaryOperator::Lt | BinaryOperator::Lte | BinaryOperator::Gt | BinaryOperator::Gte => {
                if !self.unify(lhs_ty, FELT_TYPE, ctx) && !self.unify(lhs_ty, U32_TYPE, ctx) {
                    return Err(Error::TypeMismatch);
                }
                BOOL_TYPE
            }
        };

        Ok(CheckedExprNode::Binary(CheckedBinaryNode {
            lhs: self.exprs.alloc_item(checked_lhs),
            operator: binary_node.operator,
            rhs: self.exprs.alloc_item(checked_rhs),
            type_id,
        }))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_unary(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let unary_node = ctx.expression(node).as_unary().cloned().unwrap();
        let checked_expr = self.visit_expr(unary_node.rhs, ctx)?;
        let type_id = checked_expr.ty();

        match unary_node.operator {
            UnaryOperator::Neg => {
                if !self.unify(type_id, FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch);
                }
            }
            UnaryOperator::Not => {
                if !self.unify(type_id, BOOL_TYPE, ctx) && !self.unify(type_id, FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch);
                }
            }
        }
        if !self.unify(type_id, FELT_TYPE, ctx) && !self.unify(type_id, BOOL_TYPE, ctx) {
            return Err(Error::TypeMismatch);
        }

        Ok(CheckedExprNode::Unary(CheckedUnaryNode {
            operator: unary_node.operator,
            rhs: self.exprs.alloc_item(checked_expr),
            type_id,
        }))
    }

    #[instrument(level = "debug", skip_all)]
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
        let generic_parameters = ctx.symbols[ty].generic_parameters();
        for (generic_param, generic_arg) in generic_parameters
            .iter()
            .zip(call_node.generic_parameters.iter())
        {
            let generic_arg = self.typecheck(generic_arg, ctx)?;
            if !self.unify(generic_param.clone(), generic_arg, ctx) {
                return Err(Error::TypeMismatch);
            }
        }
        let signature = ctx.symbols[ty].signature();

        if call_node.args.len() != signature.parameters.len() {
            return Err(Error::InvalidFunctionCall);
        }
        let mut args = Vec::new();
        for (i, arg) in call_node.args.iter().enumerate() {
            let type_arg = self.visit_expr(arg.clone(), ctx)?;
            if !self.unify(type_arg.ty(), signature.parameters[i], ctx) {
                return Err(Error::FunctionParameterMismatch);
            }
            args.push(type_arg);
        }

        return Ok(CheckedExprNode::Call(CheckedCallNode {
            callee: self.exprs.alloc_item(callee),
            generic_parameters: generic_parameters,
            args: self.exprs.alloc_items(args),
            type_id: self.substitute_all(signature.return_type, ctx)?,
        }));
    }

    #[instrument(level = "debug", skip_all)]
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
        for (generic_param, generic_arg) in f
            .generic_parameters
            .iter()
            .zip(call_node.generic_parameters.iter())
        {
            let generic_arg = self.typecheck(generic_arg, ctx)?;
            if !self.unify(generic_param.clone(), generic_arg, ctx) {
                return Err(Error::TypeMismatch);
            }
        }
        let mut args = Vec::new();
        // TODO: add member call
        let receiver = {
            let receiver = self.visit_expr(call_node.receiver, ctx)?;
            if let Some((&underlying_type_id, generic_args)) =
                ctx.symbols[receiver.ty()].as_generic_instance()
            {
                self.populate_generic_arguments(underlying_type_id, generic_args.to_vec(), ctx)?;
            }
            if !self.unify(receiver.ty(), f.parameters[0].2, ctx) {
                return Err(Error::FunctionParameterMismatch);
            }

            self.exprs.alloc_item(receiver)
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
            type_id: self.substitute_all(f.return_type, ctx)?,
        }));
    }

    #[instrument(level = "debug", skip_all)]
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
        let type_id = ctx
            .symbols
            .get_or_add_type(Some(scope_id), tuple_type.key(), tuple_type)?;

        let elements_with_types = checked_elements
            .into_iter()
            .map(|e| (e.ty(), self.exprs.alloc_item(e)))
            .collect();

        let checked_expr =
            CheckedExprNode::Value(CheckedValueNode::Tuple(type_id, elements_with_types));

        Ok(checked_expr)
    }
    #[instrument(level = "debug", skip_all)]
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

        if (self.unify(src_type, FELT_TYPE, ctx)
            || self.unify(src_type, BOOL_TYPE, ctx)
            || self.unify(src_type, U32_TYPE, ctx))
            && (self.unify(target_type, FELT_TYPE, ctx)
                || self.unify(target_type, BOOL_TYPE, ctx)
                || self.unify(target_type, U32_TYPE, ctx))
        {
            return Ok(CheckedExprNode::Cast(CheckedCastNode {
                value: self.exprs.alloc_item(src_expr),
                target_type,
            }));
        } else {
            return Err(Error::TypeMismatch);
        };
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_if_expr(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let if_expr_node = ctx.expression(node).as_if_expr().cloned().unwrap();
        let checked_expr = self.visit_expr(if_expr_node.if_branch.predicate, ctx)?;
        if !self.unify(checked_expr.ty(), BOOL_TYPE, ctx) {
            return Err(Error::TypeMismatch);
        }

        let checked_block = self.visit_stmt(if_expr_node.if_branch.body, ctx)?;

        let if_type = self.exprs[checked_block.as_expression().unwrap().clone()].ty();
        let if_branch = CheckedCase {
            predicate: self.exprs.alloc_item(checked_expr).clone(),
            type_id: BOOL_TYPE,
            body: self.stmts.alloc_item(checked_block),
        };

        let mut elseif_branches = Vec::with_capacity(if_expr_node.elseif_branches.len());
        for branch in &if_expr_node.elseif_branches {
            let checked_expr = self.visit_expr(branch.predicate, ctx)?;
            if !self.unify(checked_expr.ty(), BOOL_TYPE, ctx) {
                return Err(Error::TypeMismatch);
            }
            let checked_block = self.visit_stmt(branch.body, ctx)?;
            let else_if_type = self.exprs[checked_block.as_expression().unwrap().clone()].ty();
            if !self.unify(else_if_type, if_type, ctx) {
                return Err(Error::TypeMismatch);
            }

            elseif_branches.push(CheckedCase {
                predicate: self.exprs.alloc_item(checked_expr).clone(),
                type_id: BOOL_TYPE,
                body: self.stmts.alloc_item(checked_block),
            });
        }

        let else_branch = if let Some(else_branch) = if_expr_node.else_branch {
            let checked_block = self.visit_stmt(else_branch, ctx)?;
            let else_type = self.exprs[checked_block.as_expression().unwrap().clone()].ty();
            if !self.unify(else_type, if_type, ctx) {
                return Err(Error::TypeMismatch);
            }

            Some(self.stmts.alloc_item(checked_block))
        } else {
            None
        };

        Ok(CheckedExprNode::IfExpr(CheckedIfExprNode {
            if_branch,
            elseif_branches,
            else_branch,
            type_id: self.substitute_all(if_type, ctx)?,
        }))
    }

    #[instrument(level = "debug", skip_all)]
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

    // #[instrument(level = "debug", skip_all)]
    // fn visit_block_expr(
    //     &mut self,
    //     node: ExprId,
    //     ctx: &mut Self::Context,
    // ) -> std::result::Result<Self::ExprResult, Self::Error> {
    //     ctx.symbols.start_scope(ScopeKind::Block);
    //     // TODO: remove clone
    //     let block = ctx.expression(node).as_block_expr().cloned().unwrap();
    //     let current_scope_id = ctx.symbols.current_scope_id().unwrap();
    //     let mut new_stmts = Vec::with_capacity(block.stmts.len());
    //
    //     for &stmt in block.uses.iter() {
    //         let use_path = ctx.statement(stmt).as_use().cloned().unwrap();
    //         self.visit_use(&use_path, ctx)?;
    //     }
    //
    //     for (i, stmt) in block.stmts.iter().enumerate() {
    //         let checked_stmt = self.visit_stmt(stmt.clone(), ctx)?;
    //         if ctx.parent_node_type() == NodeType::FunctionDef
    //             || ctx.parent_node_type() == NodeType::LambdaFunctionExpr
    //         {
    //             if checked_stmt.is_return() && i != block.stmts.len() - 1 {
    //                 return Err(Error::InvalidReturn);
    //             }
    //         }
    //         new_stmts.push(checked_stmt);
    //     }
    //
    //     ctx.symbols.end_scope();
    //     Ok(CheckedExprNode::BlockExpr(CheckedBlockExprNode {
    //         stmts: self.stmts.alloc_items(new_stmts),
    //         type_id:
    //         scope_id: current_scope_id,
    //     }))
    // }

    #[instrument(level = "debug", skip_all)]
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
            type_id: self.substitute_all(lhs_ty, ctx)?,
        }))
    }

    #[instrument(level = "debug", skip_all)]
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
            CheckedVariable::new(rhs_ty, variable_node.qualifier, current_scope_id, None),
        )?;
        let checked_variable = CheckedVariableNode {
            name: variable_node.name,
            ty: self.substitute_all(rhs_ty, ctx)?,
            qualifier: variable_node.qualifier,
            value: self.exprs.alloc_item(checked_expr),
            scope_id: current_scope_id,
        };
        Ok(CheckedStmtNode::Variable(checked_variable))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_return(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let return_node = ctx.statement(node).as_return().cloned().unwrap();
        // let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        // let parent_scope_id = ctx.symbols.parent_scope_id().unwrap();
        // if ctx.symbols[current_scope_id].kind != ScopeKind::Block {
        //     return Err(Error::InvalidReturn);
        // }
        // let valid_kinds = [
        //     ScopeKind::LambdaFunction,
        //     ScopeKind::Function,
        //     ScopeKind::ImplMethod,
        //     ScopeKind::TraitMethod,
        //     ScopeKind::LambdaFunction,
        // ];
        // if !valid_kinds.contains(&ctx.symbols[parent_scope_id].kind) {
        //     return Err(Error::InvalidReturn);
        // }

        let ret = if let Some(expr) = return_node.0 {
            let expr = self.visit_expr(expr, ctx)?;
            Some(self.exprs.alloc_item(expr))
        } else {
            None
        };

        Ok(CheckedStmtNode::Return(CheckedReturnNode { ret }))
    }

    #[instrument(level = "debug", skip_all)]
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

    #[instrument(level = "debug", skip_all)]
    fn visit_impl(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let impl_node = ctx.definition(node).as_impl().cloned().unwrap();

        let mut checked_generic_parameters = Vec::new();
        for &generic_parameter in &impl_node.generic_parameters {
            checked_generic_parameters.push(ctx.symbols.add_type_variable(generic_parameter)?);
        }

        let underlying_type_id = ctx.symbols.get_type_id(None, impl_node.ty.name()).unwrap();
        let implementor_scope = ctx.symbols[underlying_type_id].scope_id();
        ctx.symbols.enter_scope(implementor_scope);
        ctx.symbols.start_scope(ScopeKind::Impl);
        ctx.push_inferences_context();

        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, underlying_type_id)?;
        ctx.symbols
            .add_type_id(None, IdentId::SELF, underlying_type_id)?;

        let mut methods = Vec::new();

        for (generic_parameter, generic_arg) in checked_generic_parameters.iter().zip(
            ctx.symbols[underlying_type_id]
                .generic_parameters()
                .into_iter(),
        ) {
            if !self.unify(generic_parameter.clone(), generic_arg, ctx) {
                return Err(Error::TypeMismatch);
            }
        }

        for &function_id in &impl_node.body {
            methods.push(self.typecheck_method(function_id, ctx)?);
        }

        let checked_impl = CheckedImplNode {
            generic_parameters: checked_generic_parameters,
            ty: underlying_type_id,
            body: methods,
            scope_id: ctx.symbols.current_scope_id().unwrap(),
        };
        // let ty = Type::Impl(checked_impl.clone());
        // symbols.add_type(symbols.parent_scope_id(), ty);

        ctx.pop_inferences_context();
        ctx.symbols.end_scope();
        ctx.symbols.exit_scope();
        Ok(CheckedDefinitionNode::Impl(checked_impl))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_trait(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Trait);
        // ctx.symbols.start_scope(ScopeKind::Impl);
        ctx.push_inferences_context();
        // TODO: remove clone
        let trait_node = ctx.definition(node).as_trait().cloned().unwrap();

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for &generic_parameter in &trait_node.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter)?;
            generic_parameters.push(type_id);
        }

        for &function_id in &trait_node.body {
            methods.push(CheckedDefinitionNode::Function(
                self.typecheck_trait_method(function_id, ctx)?,
            ));
        }
        let checked_trait = CheckedTraitNode {
            generic_parameters,
            name: trait_node.name,
            body: self.defs.alloc_items(methods),
            def_ids: trait_node.body.clone(),
            implementors: Vec::new(),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            visibility: trait_node.visibility,
        };
        // TODO: remove clone
        let ty = Type::Trait(checked_trait.clone());
        ctx.symbols
            .add_type(ctx.symbols.parent_scope_id(), checked_trait.name, ty)?;

        ctx.pop_inferences_context();
        // ctx.symbols.end_scope();
        ctx.symbols.end_scope();
        Ok(CheckedDefinitionNode::Trait(checked_trait))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_function(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Function);
        ctx.push_inferences_context();
        let checked_function = self.typecheck_function(node, ctx)?;
        let ty = Type::Function(checked_function.clone());
        ctx.symbols
            .add_type(ctx.symbols.parent_scope_id(), ty.name(), ty)?;
        ctx.pop_inferences_context();
        ctx.symbols.end_scope();
        Ok(CheckedDefinitionNode::Function(checked_function))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_struct(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Struct);
        ctx.push_inferences_context();
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
        }

        let ty = Type::Struct(checked_struct.clone());
        ctx.symbols
            .add_type(ctx.symbols.parent_scope_id(), checked_struct.name, ty)?;

        ctx.pop_inferences_context();
        ctx.symbols.end_scope();
        Ok(CheckedDefinitionNode::Struct(checked_struct))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_enum(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Enum);
        ctx.push_inferences_context();
        // TODO: remove clone
        let enum_node = ctx.definition(node).as_enum().cloned().unwrap();
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();

        let mut generic_parameters = Vec::new();

        for &generic_parameter in &enum_node.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter)?;
            generic_parameters.push(type_id);
        }

        let checked_enum = CheckedEnumNode {
            generic_parameters,
            name: enum_node.name,
            variants: Vec::new(),
            scope_id: current_scope_id,
            implementations: Vec::new(),
            visibility: enum_node.visibility,
        };
        let ty = Type::Enum(checked_enum.clone());
        ctx.symbols
            .add_type(ctx.symbols.parent_scope_id(), checked_enum.name, ty)?;

        ctx.pop_inferences_context();
        ctx.symbols.end_scope();
        for variant in &enum_node.variants {
            match variant {
                EnumVariant::Basic(_name) => todo!(),
                EnumVariant::Tuple(_name, _members) => todo!(),
                EnumVariant::Struct(_name, _fields) => todo!(),
            }
        }
        todo!();
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_expr(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        ctx.push_node_id(NodeId::from(expr_id));
        ctx.push_inferences();
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
            NodeType::LambdaFunctionExpr => self.visit_lambda_function(expr_id, ctx)?,
            NodeType::BlockExpr => self.visit_block_expr(expr_id, ctx)?,
            NodeType::IfExpr => self.visit_if_expr(expr_id, ctx)?,
            NodeType::TupleExpr => self.visit_tuple(expr_id, ctx)?,
            NodeType::TupleAccessExpr => self.visit_tuple_access(expr_id, ctx)?,
            _ => std::unreachable!(),
        };
        ctx.pop_inferences();
        ctx.pop_node_id();
        Ok(res)
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_definition(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        ctx.push_node_id(NodeId::from(def_id));
        let res = match ctx.definition(def_id).node_type() {
            NodeType::FunctionDef => self.visit_function(def_id, ctx)?,
            NodeType::StructDef => self.visit_struct(def_id, ctx)?,
            NodeType::EnumDef => self.visit_enum(def_id, ctx)?,
            NodeType::ImplDef => self.visit_impl(def_id, ctx)?,
            NodeType::ImplTraitDef => self.visit_impl_trait(def_id, ctx)?,
            NodeType::TraitDef => self.visit_trait(def_id, ctx)?,
            NodeType::TypeAliasDef => self.visit_type_alias(def_id, ctx)?,
            NodeType::ConstDef => self.visit_const(def_id, ctx)?,
            NodeType::UseDef => self.visit_use(def_id, ctx)?,
            _ => std::unreachable!(),
        };
        ctx.pop_node_id();
        Ok(res)
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_stmt(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        ctx.push_node_id(NodeId::from(stmt_id));
        let res = match ctx.statement(stmt_id).node_type() {
            NodeType::WhileStmt => self.visit_while(stmt_id, ctx)?,
            NodeType::ForStmt => self.visit_for(stmt_id, ctx)?,
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

        for &def_id in &module.definitions {
            self.visit_definition(def_id, ctx)?;
        }
        ctx.pop_node_id();

        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_program(&mut self, ctx: &mut Self::Context) -> std::result::Result<(), Self::Error> {
        // TODO: remove clone
        ctx.symbols
            .load_modules(ctx.program().modules.clone().iter());
        let mut colors = HashMap::new();
        ctx.dependency_graph()
            .ts(&ModuleId::root(), &mut colors, &mut |&module_id| {
                ctx.symbols.enter_module(module_id);
                self.visit_module(module_id, ctx).unwrap();
                ctx.symbols.exit_module();
            })
            .unwrap();

        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_block_expr(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Block);

        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        let BlockExprNode { stmts } = ctx.expression(node).as_block_expr().unwrap().clone();
        let mut checked_stmts = Vec::with_capacity(stmts.len());

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
            scope_id: current_scope_id,
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
        let mut key: TypeKey = node.name.into();
        key.visibility = node.visibility;
        ctx.symbols.add_type_id(None, key, type_id)?;

        Ok(CheckedDefinitionNode::TypeAlias(CheckedTypeAliasNode {
            name: node.name,
            ty: type_id,
            visibility: node.visibility,
        }))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_const(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let node = ctx.definition(node).as_const().cloned().unwrap();
        let lhs_ty = self.typecheck(&node.ty, ctx)?;
        let value = self.visit_expr(node.value, ctx)?;
        let rhs_ty = value.ty();
        if !self.unify(lhs_ty, rhs_ty, ctx) {
            return Err(Error::TypeMismatch);
        }

        let node = CheckedConstNode {
            name: node.name,
            ty: rhs_ty,
            value: self.exprs.alloc_item(value),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            visibility: node.visibility,
        };

        let ty = Type::Const(node.clone());

        ctx.symbols.add_type(None, ty.key(), ty)?;

        Ok(CheckedDefinitionNode::Const(node))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_for(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let for_node = ctx.statement(node).as_for().cloned().unwrap();
        ctx.symbols.start_scope(ScopeKind::Block);
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        let variable =
            CheckedVariable::new(FELT_TYPE, TypeQualifier::new(true), current_scope_id, None);
        ctx.symbols.declare_variable(for_node.variable, variable)?;
        let start = self.visit_expr(for_node.start, ctx)?;
        let end = self.visit_expr(for_node.end, ctx)?;
        if !self.unify(start.ty(), FELT_TYPE, ctx) || !self.unify(end.ty(), FELT_TYPE, ctx) {
            return Err(Error::TypeMismatch);
        }
        ctx.symbols.start_scope(ScopeKind::Block);
        let checked_block = self.visit_stmt(for_node.body, ctx)?;
        let node = CheckedStmtNode::For(CheckedForNode {
            variable: for_node.variable,
            start: self.exprs.alloc_item(start),
            end: self.exprs.alloc_item(end),
            body: self.stmts.alloc_item(checked_block),
            scope_id: current_scope_id,
        });
        ctx.symbols.end_scope();
        ctx.symbols.end_scope();
        Ok(node)
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_match(
        &mut self,
        _node: StmtId,
        _ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        todo!()
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_lambda_function(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        ctx.symbols.start_scope(ScopeKind::LambdaFunction);
        let function = ctx.expression(node).as_lambda_function().cloned().unwrap();

        let current_scope_id = ctx.symbols.current_scope_id().unwrap();

        let mut parameters = Vec::new();

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

        let checked_body = {
            let checked_body = self.visit_stmt(function.body.clone(), ctx)?;
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
            if !self.unify(expected_return_type, actual_return_type, ctx) {
                return Err(Error::TypeMismatch);
            }

            checked_body
        };

        let mut checked_function = CheckedLambdaFunctionNode {
            name: ctx.intern_lambda(),
            parameters,
            body: self.stmts.alloc_item(checked_body),
            return_type: expected_return_type,
            scope_id: current_scope_id,
            type_id: UNKOWN_TYPE,
        };

        let ty = Type::LambdaFunction(checked_function.clone());
        let type_id = ctx
            .symbols
            .add_type(ctx.symbols[current_scope_id].parent, ty.key(), ty)?;
        (ctx.symbols[type_id].as_mut() as &mut CheckedLambdaFunctionNode).type_id = type_id;
        checked_function.type_id = type_id;

        ctx.symbols.end_scope();
        Ok(CheckedExprNode::LambdaFunction(checked_function))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_impl_trait(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let impl_node = ctx.definition(node).as_impl_trait().cloned().unwrap();

        let trait_type_id = self.typecheck(&impl_node.trait_ty, ctx)?;
        let implementor_type_id = ctx.symbols.get_type_id(None, impl_node.ty.name()).unwrap();

        ctx.symbols
            .enter_scope(ctx.symbols[trait_type_id].scope_id());
        ctx.symbols.start_scope(ScopeKind::Impl);
        ctx.push_inferences_context();

        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, implementor_type_id)?;
        ctx.symbols.add_type_id(
            None,
            ctx.symbols[implementor_type_id].key(),
            implementor_type_id,
        )?;
        ctx.symbols
            .add_type_id(None, IdentId::SELF, implementor_type_id)?;

        let trait_node = ctx.symbols[trait_type_id].clone().into_trait().unwrap();
        let mut generic_parameters = Vec::new();
        let mut methods = vec![Default::default(); trait_node.body.len()];

        for &generic_parameter in &impl_node.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter)?;
            generic_parameters.push(type_id);
        }

        for &function_id in &impl_node.body {
            let method = self.typecheck_method(function_id, ctx)?;
            let i = trait_node
                .body
                .iter()
                .position(|trait_def_id| {
                    let trait_method = self.defs[*trait_def_id].clone().into_function().unwrap();
                    trait_method.trait_impl_signature(implementor_type_id) == method.signature()
                })
                .ok_or(Error::UnresolvedTraitMethod)?;
            methods[i] = method;
        }

        for i in 0..methods.len() {
            if methods[i] == Default::default() {
                let trait_method = self.defs[trait_node.body[i]]
                    .clone()
                    .into_function()
                    .unwrap();
                if trait_method.body.is_none() {
                    return Err(Error::UnresolvedTraitMethod);
                }
                methods[i] = self.typecheck_method(trait_node.def_ids[i], ctx)?;
            }
        }

        let checked_impl = CheckedImplTraitNode {
            generic_parameters,
            trait_ty: trait_type_id,
            ty: implementor_type_id,
            body: methods,
            scope_id: ctx.symbols.current_scope_id().unwrap(),
        };
        // let ty = Type::Impl(checked_impl.clone());
        // symbols.add_type(symbols.parent_scope_id(), ty);

        ctx.symbols
            .impl_trait_for_type(trait_type_id, implementor_type_id);

        ctx.pop_inferences_context();
        ctx.symbols.end_scope();
        ctx.symbols.exit_scope();
        Ok(CheckedDefinitionNode::ImplTrait(checked_impl))
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
        #[allow(static_mut_refs)]
        unsafe {
            STD_PRIMITIVE_SCOPE_ID
                .set(ctx.symbols.current_scope_id().unwrap())
                .unwrap()
        };
        for ty in &*PRIMITIVE_TYPES {
            ctx.symbols.add_type(None, ty.key(), ty.clone())?;
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_member_access(
        &mut self,
        receiver: ExprId,
        ctx: &TypeCheckerVisitorContext<F, C>,
    ) -> bool {
        ctx.symbols
            .find(None, vec![ScopeKind::Impl], |s| {
                s.kind.eq(&ScopeKind::ImplMethod).then_some(true)
            })
            .is_some()
            && ctx
                .expression(receiver)
                .as_path()
                .map(|x| x.is_receiver())
                .unwrap_or(false)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn populate_generic_arguments(
        &mut self,
        underlying_type_id: TypeId,
        generic_args: Vec<TypeId>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        for (generic_param, generic_arg) in ctx.symbols[underlying_type_id]
            .generic_parameters()
            .into_iter()
            .zip(generic_args.clone().into_iter())
        {
            if !self.unify(generic_param, generic_arg, ctx) {
                return Err(Error::TypeMismatch);
            }
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn typecheck_method_receiver(
        &mut self,
        function: &FunctionNode,
        ctx: &TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        let is_method = [ScopeKind::ImplMethod, ScopeKind::TraitMethod]
            .contains(&ctx.symbols[current_scope_id].kind);

        for (i, (parameter, _, parameter_type)) in function.parameters.iter().enumerate() {
            if parameter == &IdentId::SELF && (i != 0 || !is_method) {
                return Err(Error::InvalidSelfParameter);
            }

            if parameter_type == &UncheckedType::Basic(IdentId::TYPE_SELF) && !is_method {
                return Err(Error::InvalidSelfParameter);
            }
        }

        Ok(())
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
            UncheckedType::Basic(IdentId::TYPE_U32) => Ok(U32_TYPE),
            UncheckedType::Basic(name) => Ok(ctx
                .symbols
                .get_type_id(None, name.clone())
                .ok_or(Error::UnresolvedType)?),
            UncheckedType::Generic(name, generic_parameters) => {
                let underlying_type_id = ctx
                    .symbols
                    .get_type_id(None, name.clone())
                    .ok_or(Error::UnresolvedType)?;

                let mut checked_generic_parameters = Vec::new();
                for generic_parameter in generic_parameters {
                    checked_generic_parameters.push(self.typecheck(generic_parameter, ctx)?);
                }

                match &ctx.symbols[underlying_type_id] {
                    Type::Struct(checked_struct) => {
                        if checked_struct.generic_parameters.len()
                            != checked_generic_parameters.len()
                        {
                            return Err(Error::GenericParameterMismatch);
                        }

                        let ty =
                            Type::GenericInstance(underlying_type_id, checked_generic_parameters);
                        let type_id = ctx.symbols.get_or_add_type(None, ty.key(), ty)?;

                        Ok(type_id)
                    }
                    Type::Enum(checked_enum) => {
                        if checked_enum.generic_parameters.len() != checked_generic_parameters.len()
                        {
                            return Err(Error::GenericParameterMismatch);
                        }

                        todo!()
                    }
                    Type::Function(checked_function) => {
                        if checked_function.generic_parameters.len()
                            != checked_generic_parameters.len()
                        {
                            return Err(Error::GenericParameterMismatch);
                        }

                        todo!()
                    }
                    Type::Trait(checked_trait) => {
                        if checked_trait.generic_parameters.len()
                            != checked_generic_parameters.len()
                        {
                            return Err(Error::GenericParameterMismatch);
                        }
                        todo!()
                    }
                    _ => unreachable!(),
                }
            }
            UncheckedType::Array(inner, size) => {
                let inner_ty = self.typecheck(inner, ctx)?;
                let scope_id = ScopeId::primitive();
                let ty = Type::Array(CheckedArrayNode {
                    inner_ty,
                    size: size.clone(),
                });
                ctx.symbols.get_or_add_type(Some(scope_id), ty.key(), ty)
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

                ctx.symbols
                    .get_or_add_type(Some(scope_id), checked_tuple.key(), checked_tuple)
            }
            UncheckedType::Unknown => Ok(UNKOWN_TYPE),
            UncheckedType::FunctionSignature(function_signature) => {
                let mut parameters = Vec::with_capacity(function_signature.parameters.len());
                for parameter_ty in &function_signature.parameters {
                    let ty = self.typecheck(parameter_ty, ctx)?;
                    parameters.push(ty);
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

                ctx.symbols.get_or_add_type(None, ty.key(), ty)
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_trait_method(
        &mut self,
        function_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        ctx.symbols.start_scope(ScopeKind::TraitMethod);
        ctx.push_inferences();

        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, UNKOWN_TYPE)?;

        let checked_function = self.typecheck_function(function_id, ctx)?;
        let ty = Type::Function(checked_function.clone());
        ctx.symbols.add_type(None, checked_function.name, ty)?;

        ctx.pop_inferences();
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
        ctx.push_inferences();

        let checked_function = self.typecheck_function(function_id, ctx)?;
        let ty = Type::Function(checked_function.clone());
        ctx.symbols.add_type(None, checked_function.name, ty)?;

        ctx.pop_inferences();
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
        let is_method = [ScopeKind::ImplMethod, ScopeKind::TraitMethod]
            .contains(&ctx.symbols[current_scope_id].kind);

        let mut generic_parameters = Vec::new();
        let mut parameters = Vec::new();

        self.typecheck_method_receiver(&function, ctx)?;

        for &generic_parameter in &function.generic_parameters {
            // TODO: fix trait default method
            let end_scope_kind = if is_method {
                ScopeKind::Impl
            } else {
                ScopeKind::Function
            };
            let type_id = ctx
                .symbols
                .get_or_add_type_variable(end_scope_kind, generic_parameter)?;
            generic_parameters.push(type_id);
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
            if !self.unify(expected_return_type, actual_return_type, ctx) {
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

    fn unify(
        &self,
        lhs_ty: TypeId,
        rhs_ty: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> bool {
        let lhs_ty = self.substitute_all(lhs_ty, ctx).unwrap();
        let rhs_ty = self.substitute_all(rhs_ty, ctx).unwrap();

        match (&ctx.symbols[lhs_ty], &ctx.symbols[rhs_ty]) {
            (Type::TypeVariable(_), _) => {
                ctx.add_inference(lhs_ty, rhs_ty);
                true
            }
            (_, Type::TypeVariable(_)) => {
                ctx.add_inference(rhs_ty, lhs_ty);
                true
            }
            (Type::Struct(s1), Type::Struct(s2)) => {
                s1.name == s2.name
                    && s1.scope_id == s2.scope_id
                    && s1.generic_parameters.len() == s2.generic_parameters.len()
                    && s1
                        .generic_parameters
                        .clone()
                        .into_iter()
                        .zip(s2.generic_parameters.clone().into_iter())
                        .all(|(p1, p2)| self.unify(p1, p2, ctx))
            }
            (Type::Array(a1), Type::Array(a2)) => {
                a1.size == a2.size && self.unify(a1.inner_ty, a2.inner_ty, ctx)
            }

            (Type::GenericInstance(underlying_type_id, args), Type::Struct(checked_struct))
            | (Type::Struct(checked_struct), Type::GenericInstance(underlying_type_id, args)) => {
                let other_ty = ctx.symbols[*underlying_type_id].clone();
                other_ty.is_struct()
                    && other_ty.name() == checked_struct.name
                    && other_ty.scope_id() == checked_struct.scope_id
                    && args
                        .clone()
                        .into_iter()
                        .zip(checked_struct.generic_parameters.clone().into_iter())
                        .all(|(p1, p2)| self.unify(p1, p2, ctx))
            }

            (Type::GenericInstance(lhs_ty, lhs_args), Type::GenericInstance(rhs_ty, rhs_args)) => {
                if lhs_ty != rhs_ty {
                    return false;
                }

                if lhs_args.len() != rhs_args.len() {
                    return false;
                }

                for (lhs_arg, rhs_arg) in lhs_args
                    .clone()
                    .into_iter()
                    .zip(rhs_args.clone().into_iter())
                {
                    if !self.unify(lhs_arg, rhs_arg, ctx) {
                        return false;
                    }
                }

                true
            }

            (Type::Function(f), Type::FunctionSignature(sig))
            | (Type::FunctionSignature(sig), Type::Function(f)) => &f.signature() == sig,
            (Type::LambdaFunction(f), Type::FunctionSignature(sig))
            | (Type::FunctionSignature(sig), Type::LambdaFunction(f)) => &f.signature() == sig,
            (Type::Const(c), Type::Felt(_)) => c.ty == rhs_ty,
            (Type::Felt(_), Type::Const(c)) => c.ty == lhs_ty,
            (Type::Const(c), Type::Const(d)) => c.ty == d.ty,
            (Type::Unknown, Type::Unknown) => false,
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            _ => lhs_ty == rhs_ty,
        }
    }

    fn substitute_all(
        &self,
        type_id: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        if !ctx.has_inferences() {
            return Ok(type_id);
        }

        let mut result = self.substitute_type(type_id, ctx)?;

        loop {
            let fixed_point = self.substitute_type(type_id, ctx)?;

            if fixed_point == result {
                break;
            } else {
                result = fixed_point;
            }
        }

        Ok(result)
    }

    fn substitute_type(
        &self,
        type_id: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        if let Some(subst_type) = ctx.resolve_type(type_id) {
            return Ok(subst_type);
        }

        match ctx.symbols[type_id].clone() {
            Type::TypeVariable(_) => Ok(type_id),

            Type::Array(array) => {
                let inner_ty = self.substitute_all(array.inner_ty, ctx)?;
                let ty = Type::Array(CheckedArrayNode {
                    inner_ty,
                    size: array.size,
                });
                ctx.symbols
                    .get_or_add_type(Some(ScopeId::primitive()), ty.key(), ty)
            }

            Type::GenericInstance(underlying_type_id, generic_parameters) => {
                let mut new_generic_parameters = Vec::new();
                for generic_parameter in generic_parameters {
                    new_generic_parameters.push(self.substitute_all(generic_parameter, ctx)?);
                }

                let ty = Type::GenericInstance(underlying_type_id, new_generic_parameters);
                ctx.symbols.get_or_add_type(None, ty.key(), ty)
            }

            _ => Ok(type_id),
        }
    }
}
