mod definition;
mod expr;
mod infer;
mod program;
mod stmt;
mod symbol_table;
mod traits;
mod r#type;
mod value;
mod variable;

mod error;
mod visualizer;

pub use definition::*;
pub use error::*;
pub use expr::*;
pub use infer::*;
pub use program::*;
pub use r#type::*;
use regex::Regex;
pub use stmt::*;
pub use symbol_table::*;
pub use traits::*;
pub use value::*;
pub use variable::*;
pub use visualizer::*;

use indexmap::IndexMap;
use qed_ast::*;
use qed_common::Graph;
use qedlang_core::dpn::ops::context_trait::ContextFelt;
use std::collections::{HashMap, HashSet};
use tracing::instrument;

pub struct TypeCheckerVisitorContext<F: Clone + From<u32> + ContextFelt, C> {
    path_stack: Vec<NodeId>,
    pub program: Program<F>,
    pub symbols: SymbolTable<F>,
    infcx: InferCtxt,
    _marker: std::marker::PhantomData<(F, C)>,
}

impl<F: Clone + From<u32> + ContextFelt, C> TypeCheckerVisitorContext<F, C> {
    pub fn new(program: Program<F>) -> Self {
        TypeCheckerVisitorContext {
            path_stack: vec![],
            program,
            symbols: SymbolTable::new(),
            infcx: InferCtxt::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn get_type_detail(&self, type_id: TypeId) -> String {
        match &self.symbols[type_id] {
            Type::Unknown => format!("Unknown"),
            Type::VOID => format!("void"),
            Type::Felt(_checked_felt_node) => format!("Felt"),
            Type::Bool(_checked_bool_node) => format!("Bool"),
            Type::U32(_checked_u32_node) => format!("U32"),
            Type::Array(checked_array_node) => {
                format!(
                    "[{}; {}]",
                    self.get_type_detail(checked_array_node.inner_ty),
                    self.get_type_detail(checked_array_node.size_ty),
                )
            }
            Type::Struct(checked_struct_node) => {
                format!("Struct {}", self.ident(checked_struct_node.name))
            }
            Type::Enum(checked_enum_node) => format!("Enum {}", self.ident(checked_enum_node.name)),
            Type::Function(checked_function_node) => {
                format!("fn {}", self.ident(checked_function_node.name))
            }
            Type::Trait(checked_trait_node) => {
                format!("Trait {}", self.ident(checked_trait_node.name))
            }
            Type::Const(checked_const_node) => {
                format!(
                    "Const {}",
                    self.ident(checked_const_node.name.unwrap_or(IdentId::TYPE_VOID))
                )
            }
            Type::LambdaFunction(checked_lambda_function_node) => {
                format!("lamba fn {}", self.ident(checked_lambda_function_node.name))
            }
            Type::FunctionSignature(_checked_function_signature) => format!("fn sig"),
            Type::TypeVariable(type_variable_node) => {
                let mut type_variable_details = vec![];
                for type_id in type_variable_node.constraints.iter() {
                    type_variable_details.push(self.get_type_detail(type_id.clone()));
                }
                format!(": {}", type_variable_details.join(" + "))
            }
            Type::Tuple(type_ids) => {
                let mut tuple_details = vec![];
                for type_id in type_ids {
                    tuple_details.push(self.get_type_detail(*type_id));
                }
                format!("({})", tuple_details.join(", "))
            }
            Type::GenericInstance(type_id, _type_ids, _scope_id) => {
                format!("<{}>", self.get_type_detail(*type_id))
            }
        }
    }

    //warn: debug only
    pub fn print_symbol_table_to_string(&self) {
        let debug_output = format!("{}", self.symbols);

        let ident_regex = Regex::new(r"IdentId\((\d+)\)").unwrap();

        // parse all `IdentId(NUM)` to `NUM`
        let mut id_to_name = HashMap::new();
        for capture in ident_regex.captures_iter(&debug_output) {
            let ident_id_str = &capture[1]; // get the number
            if let Ok(ident_id) = ident_id_str.parse::<usize>() {
                let ident_name = self.program[IdentId(ident_id)].clone();
                id_to_name.insert(ident_id_str.to_string(), ident_name);
            }
        }

        // replace all `IdentId(NUM)` to `IdentId(NUM: "name")`
        let formatted_output = ident_regex.replace_all(&debug_output, |caps: &regex::Captures| {
            let ident_id_str = &caps[1];
            if let Some(name) = id_to_name.get(ident_id_str) {
                format!("IdentId({}: \"{}\")", ident_id_str, name)
            } else {
                caps[0].to_string()
            }
        });
        println!("Symbol Table \n{}", formatted_output);
    }
}

impl<F: Clone + From<u32> + ContextFelt, C> VisitorContext<F, C>
    for TypeCheckerVisitorContext<F, C>
{
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

pub struct TypeChecker<F: Clone + From<u32> + ContextFelt, C> {
    pub program: CheckedProgram<F>,
    evaluator: Box<dyn Evaluator<F, C>>,
    _marker: std::marker::PhantomData<C>,
}

impl<F: Clone + From<u32> + ContextFelt, C> AstVisitor<F, C> for TypeChecker<F, C> {
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

        let formatted_use_path = {
            let use_node = node.clone();
            let mut path = vec![ctx.ident(use_node.kind).to_string()];
            let segments = use_node
                .segments
                .iter()
                .map(|&s| ctx.ident(s).to_string())
                .collect::<Vec<_>>();
            path.extend(segments);
            let target = use_node
                .target
                .map(|t| ctx.ident(t).to_string())
                .unwrap_or("*".to_string());
            path.push(target);
            format!("{}", path.join("::"))
        };

        ctx.symbols
            .add_use(&node)
            .ok_or(error::Error::UnresolvedUse {
                span: ctx.program.convert_span(&node.span),
                resolved_use: formatted_use_path,
            })?;
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
                span: path_node.span,
            }));
        } else {
            return Err(Error::UnresolvedPath {
                span: ctx.program.convert_span(&path_node.span),
                resolved_path: ctx.ident(path_node.target).to_string(),
            });
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
        let ty = &ctx.symbols[type_id];

        let inner_ty = if let Some(a) = ty.as_array() {
            a.inner_ty
        } else {
            let (&underlying_type_id, generic_args, _) = ty.as_generic_instance().unwrap();
            let innner_ty = generic_args.first().unwrap().clone();
            self.populate_generic_arguments(underlying_type_id, generic_args.to_vec(), ctx)?;
            innner_ty
        };

        if !self.unify(checked_index.ty(), FELT_TYPE, ctx) {
            return Err(Error::TypeMismatch {
                span: ctx.program.convert_span(&index_access_node.span),
                expected: ctx.get_type_detail(FELT_TYPE),
                found: ctx.get_type_detail(checked_index.ty()),
            });
        }

        Ok(CheckedExprNode::IndexAccess(CheckedIndexAccessNode {
            target: self.program.exprs.alloc_item(checked_expr),
            index: self.program.exprs.alloc_item(checked_index),
            type_id: self.substitute_all(inner_ty, ctx)?,
            span: index_access_node.span,
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

        let underlying_type_id = if let Some(_) = ty.as_struct() {
            type_id
        } else {
            let (&underlying_type_id, generic_args, _) = ty.as_generic_instance().unwrap();
            self.populate_generic_arguments(underlying_type_id, generic_args.to_vec(), ctx)?;
            underlying_type_id
        };

        if ctx.parent_node_type().is_member_call_expr() {
            let type_id = ctx
                .symbols
                .resolve_method(underlying_type_id, member_access_node.field)
                .ok_or(Error::UnresolvedMember {
                    span: ctx.program.convert_span(&member_access_node.span),
                    member_name: format!("{}()", ctx.ident(member_access_node.field)),
                })?;
            let visibility = ctx.symbols[type_id].visibility();
            assert!(
                visibility.is_public()
                    || self.typecheck_member_access(member_access_node.target, ctx)
            );
            return Ok(CheckedExprNode::MemberAccess(CheckedMemberAccessNode {
                target: self.program.exprs.alloc_item(checked_expr),
                field: member_access_node.field,
                type_id,
                span: member_access_node.span,
            }));
        } else {
            let fields = &ctx.symbols[underlying_type_id].as_struct().unwrap().fields;
            let (field_type, visibility) =
                fields
                    .get(&member_access_node.field)
                    .ok_or(Error::UnresolvedMember {
                        span: ctx.program.convert_span(&member_access_node.span),
                        member_name: format!("{}", ctx.ident(member_access_node.field)),
                    })?;
            assert!(
                visibility.is_public()
                    || self.typecheck_member_access(member_access_node.target, ctx)
            );
            return Ok(CheckedExprNode::MemberAccess(CheckedMemberAccessNode {
                target: self.program.exprs.alloc_item(checked_expr),
                field: member_access_node.field.clone(),
                type_id: self.substitute_all(field_type.clone(), ctx)?,
                span: member_access_node.span,
            }));
        }
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
        let element_types = ty.as_tuple().ok_or(Error::TypeMismatch {
            span: ctx.program.convert_span(&tuple_access_node.span),
            expected: format!("Tuple"),
            found: ctx.get_type_detail(type_id),
        })?;

        if tuple_access_node.index >= element_types.len() {
            return Err(Error::IndexOutOfBounds {
                span: ctx.program.convert_span(&tuple_access_node.span),
                index: tuple_access_node.index,
                length: element_types.len(),
            });
        }

        let field_type = element_types
            .get(tuple_access_node.index)
            .ok_or(Error::IndexOutOfBounds {
                span: ctx.program.convert_span(&tuple_access_node.span),
                index: tuple_access_node.index,
                length: element_types.len(),
            })?
            .clone();
        Ok(CheckedExprNode::TupleAccess(CheckedTupleAccessNode {
            target: self.program.exprs.alloc_item(checked_expr),
            index: tuple_access_node.index,
            type_id: field_type,
            span: tuple_access_node.span,
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
                    return Err(Error::TypeMismatch {
                        span: FileSpan::default(),
                        expected: ctx.get_type_detail(FELT_TYPE),
                        found: ctx.get_type_detail(slot_index.ty()),
                    });
                }
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetStateHashAt {
                        slot_index: self.program.exprs.alloc_item(slot_index),
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
                    return Err(Error::TypeMismatch {
                        span: FileSpan::default(),
                        expected: ctx.get_type_detail(FELT_TYPE),
                        found: format!(
                            "contract_state_tree_height: {}, contract_id: {} slot_index: {}",
                            ctx.get_type_detail(contract_state_tree_height.ty()),
                            ctx.get_type_detail(contract_id.ty()),
                            ctx.get_type_detail(slot_index.ty()),
                        ),
                    });
                }

                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetOtherContractStateHashAt {
                        contract_state_tree_height: self
                            .program
                            .exprs
                            .alloc_item(contract_state_tree_height),
                        contract_id: self.program.exprs.alloc_item(contract_id),
                        slot_index: self.program.exprs.alloc_item(slot_index),
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
                    return Err(Error::TypeMismatch {
                        span: FileSpan::default(),
                        expected: ctx.get_type_detail(FELT_TYPE),
                        found: format!(
                            "contract_state_tree_height: {}, user_id: {}, contract_id: {} slot_index: {}",
                            ctx.get_type_detail(contract_state_tree_height.ty()),
                            ctx.get_type_detail(user_id.ty()),
                            ctx.get_type_detail(contract_id.ty()),
                            ctx.get_type_detail(slot_index.ty()),
                        ),
                    });
                }

                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetOtherUserContractStateHashAt {
                        contract_state_tree_height: self
                            .program
                            .exprs
                            .alloc_item(contract_state_tree_height),
                        user_id: self.program.exprs.alloc_item(user_id),
                        contract_id: self.program.exprs.alloc_item(contract_id),
                        slot_index: self.program.exprs.alloc_item(slot_index),
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
                    return Err(Error::TypeMismatch {
                        span: FileSpan::default(),
                        expected: format!("Felt, Hash"),
                        found: format!(
                            "slot_index: {}, new_value: {}",
                            ctx.get_type_detail(slot_index.ty()),
                            ctx.get_type_detail(new_value.ty()),
                        ),
                    });
                }

                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::CSetStateHashAt {
                        slot_index: self.program.exprs.alloc_item(slot_index),
                        new_value: self.program.exprs.alloc_item(new_value),
                        type_id: HASH_TYPE,
                    },
                ));
            }
            IntrinsicExprNode::Read { offset } => {
                // TODO: remove clone
                let offset = self.visit_expr(offset, ctx)?;
                if !self.unify(offset.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: FileSpan::default(),
                        expected: ctx.get_type_detail(FELT_TYPE),
                        found: ctx.get_type_detail(offset.ty()),
                    });
                }
                return Ok(CheckedExprNode::Intrinsic(CheckedIntrinsicExprNode::Read {
                    offset: self.program.exprs.alloc_item(offset),
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
                    return Err(Error::TypeMismatch {
                        span: FileSpan::default(),
                        expected: ctx.get_type_detail(FELT_TYPE),
                        found: format!(
                            "offset: {}, value: {}",
                            ctx.get_type_detail(offset.ty()),
                            ctx.get_type_detail(value.ty()),
                        ),
                    });
                }
                Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::Write {
                        offset: self.program.exprs.alloc_item(offset),
                        value: self.program.exprs.alloc_item(value),
                        type_id: FELT_TYPE,
                    },
                ))
            }
            IntrinsicExprNode::Hash { data } => {
                let data = self.visit_expr(data, ctx)?;

                Ok(CheckedExprNode::Intrinsic(CheckedIntrinsicExprNode::Hash {
                    data: self.program.exprs.alloc_item(data),
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
            ValueNode::Felt(f, _span) => {
                Ok(CheckedExprNode::Value(CheckedValueNode::Felt(f.clone())))
            }
            ValueNode::Bool(b, _span) => {
                Ok(CheckedExprNode::Value(CheckedValueNode::Bool(b.clone())))
            }
            ValueNode::U32(u, _span) => {
                Ok(CheckedExprNode::Value(CheckedValueNode::U32(u.clone())))
            }
            ValueNode::Array(size, arr, _span) => {
                if size != arr.len() {
                    return Err(Error::TypeMismatch {
                        span: FileSpan::default(),
                        expected: format!("{} size array", size),
                        found: format!("{}", arr.len(),),
                    });
                }

                let mut inner_ty = UNKOWN_TYPE;
                let mut elements = Vec::with_capacity(arr.len());
                for e in arr {
                    // TODO: remove clone
                    let checked_expr = self.visit_expr(e, ctx)?;
                    if !self.unify(checked_expr.ty(), inner_ty, ctx) {
                        return Err(Error::TypeMismatch {
                            span: FileSpan::default(),
                            expected: ctx.get_type_detail(inner_ty),
                            found: ctx.get_type_detail(checked_expr.ty()),
                        });
                    }
                    inner_ty = checked_expr.ty();
                    elements.push(self.program.exprs.alloc_item(checked_expr));
                }

                let underlying_type_id = ctx
                    .symbols
                    .get_type_id(Some(ScopeId::primitive()), IdentId::TYPE_ARRAY)
                    .unwrap();

                let size_ty = self.populate_constant_u32(size, ctx)?;

                let scope_id = ScopeId::primitive();
                let ty = Type::GenericInstance(
                    underlying_type_id,
                    vec![self.substitute_all(inner_ty, ctx)?, size_ty],
                    scope_id,
                );
                let type_id = ctx.symbols.get_or_add_type(Some(scope_id), ty.key(), ty)?;

                Ok(CheckedExprNode::Value(CheckedValueNode::Array(
                    type_id, elements,
                )))
            }
            ValueNode::Struct(name, generic_args, data, span) => Ok({
                let underlying_type_id =
                    ctx.symbols
                        .get_type_id(None, name)
                        .ok_or(Error::UnresolvedType {
                            span: ctx.program.convert_span(&span),
                            resolved_type: format!("{}", ctx.ident(name)),
                        })?;
                let fields = ctx.symbols[underlying_type_id]
                    .as_struct()
                    .unwrap()
                    .fields
                    .clone();
                let generic_parameters = ctx.symbols[underlying_type_id].generic_parameters();
                if fields.len() != data.len() {
                    return Err(Error::TypeMismatch {
                        span: FileSpan::default(),
                        expected: format!("{} fields for Struct {}", fields.len(), ctx.ident(name)),
                        found: format!("{}", data.len(),),
                    });
                }

                let mut new_data = IndexMap::new();
                for (field_name, (field_type, _)) in fields {
                    let field_value =
                        self.visit_expr(data.get(&field_name).unwrap().clone(), ctx)?;
                    if !self.unify(field_type, field_value.ty(), ctx) {
                        return Err(Error::TypeMismatch {
                            span: FileSpan::default(),
                            expected: ctx.get_type_detail(field_type),
                            found: ctx.get_type_detail(field_value.ty()),
                        });
                    }
                    new_data.insert(field_name, self.program.exprs.alloc_item(field_value));
                }

                for (generic_arg, generic_param) in generic_args
                    .clone()
                    .iter()
                    .zip(generic_parameters.clone().into_iter())
                {
                    let generic_arg = self.typecheck(generic_arg, ctx)?;
                    if !self.unify(generic_param, generic_arg, ctx) {
                        return Err(Error::TypeMismatch {
                            span: FileSpan::default(),
                            expected: ctx.get_type_detail(generic_arg),
                            found: ctx.get_type_detail(generic_param),
                        });
                    }
                }

                let type_id = if generic_parameters.is_empty() {
                    underlying_type_id
                } else {
                    let ty = Type::GenericInstance(
                        underlying_type_id,
                        generic_parameters,
                        ctx.symbols.current_scope_id().unwrap(),
                    );

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
            return Err(Error::TypeMismatch {
                span: ctx.program.convert_span(&binary_node.span),
                expected: format!(
                    "{} for {}",
                    ctx.get_type_detail(lhs_ty),
                    binary_node.operator
                ),
                found: ctx.get_type_detail(checked_rhs.ty()),
            });
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
                    return Err(Error::TypeMismatch {
                        span: ctx.program.convert_span(&binary_node.span),
                        expected: format!("Felt, U32 for {}", binary_node.operator),
                        found: ctx.get_type_detail(lhs_ty),
                    });
                }
            }
            BinaryOperator::And | BinaryOperator::Or => {
                if !self.unify(lhs_ty, BOOL_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: ctx.program.convert_span(&binary_node.span),
                        expected: format!("Bool for {}", binary_node.operator),
                        found: ctx.get_type_detail(lhs_ty),
                    });
                }
                BOOL_TYPE
            }
            BinaryOperator::Eq | BinaryOperator::Neq => {
                if !self.unify(lhs_ty, BOOL_TYPE, ctx)
                    && !self.unify(lhs_ty, FELT_TYPE, ctx)
                    && !self.unify(lhs_ty, U32_TYPE, ctx)
                {
                    return Err(Error::TypeMismatch {
                        span: ctx.program.convert_span(&binary_node.span),
                        expected: format!("Felt, U32, Bool for {}", binary_node.operator),
                        found: ctx.get_type_detail(lhs_ty),
                    });
                }
                BOOL_TYPE
            }
            BinaryOperator::Lt | BinaryOperator::Lte | BinaryOperator::Gt | BinaryOperator::Gte => {
                if !self.unify(lhs_ty, FELT_TYPE, ctx) && !self.unify(lhs_ty, U32_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: ctx.program.convert_span(&binary_node.span),
                        expected: format!("Felt, U32 for {}", binary_node.operator),
                        found: ctx.get_type_detail(lhs_ty),
                    });
                }
                BOOL_TYPE
            }
        };

        Ok(CheckedExprNode::Binary(CheckedBinaryNode {
            lhs: self.program.exprs.alloc_item(checked_lhs),
            operator: binary_node.operator,
            rhs: self.program.exprs.alloc_item(checked_rhs),
            type_id,
            span: binary_node.span,
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
                    return Err(Error::TypeMismatch {
                        span: ctx.program.convert_span(&unary_node.span),
                        expected: format!(
                            "{} for {}",
                            ctx.get_type_detail(FELT_TYPE),
                            unary_node.operator
                        ),
                        found: ctx.get_type_detail(type_id),
                    });
                }
            }
            UnaryOperator::Not => {
                if !self.unify(type_id, BOOL_TYPE, ctx) && !self.unify(type_id, FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: ctx.program.convert_span(&unary_node.span),
                        expected: format!("Bool, Felt for {}", unary_node.operator),
                        found: ctx.get_type_detail(type_id),
                    });
                }
            }
        }
        if !self.unify(type_id, FELT_TYPE, ctx) && !self.unify(type_id, BOOL_TYPE, ctx) {
            return Err(Error::TypeMismatch {
                span: ctx.program.convert_span(&unary_node.span),
                expected: format!("Felt, Bool"),
                found: format!("{:?}", ctx.symbols[type_id].kind()),
            });
        }

        Ok(CheckedExprNode::Unary(CheckedUnaryNode {
            operator: unary_node.operator,
            rhs: self.program.exprs.alloc_item(checked_expr),
            type_id,
            span: unary_node.span,
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
        let generic_parameters = ctx.symbols[ty].generic_parameters();
        for (generic_param, generic_arg) in generic_parameters
            .iter()
            .zip(call_node.generic_parameters.iter())
        {
            let generic_arg = self.typecheck(generic_arg, ctx)?;
            if !self.unify(generic_param.clone(), generic_arg, ctx) {
                return Err(Error::TypeMismatch {
                    span: ctx.program.convert_span(&call_node.span),
                    expected: ctx.get_type_detail(generic_param.clone()),
                    found: ctx.get_type_detail(generic_arg),
                });
            }
        }

        let signature = ctx.symbols[ty].signature();

        if call_node.args.len() != signature.parameters.len() {
            return Err(Error::InvalidFunctionCall {
                span: ctx.program.convert_span(&call_node.span),
                method_name: ctx.get_type_detail(ty),
                expected: format!("{} parameters", signature.parameters.len()),
                found: format!("{}", call_node.args.len()),
            });
        }
        let mut args = Vec::new();
        for (i, arg) in call_node.args.iter().enumerate() {
            let type_arg = self.visit_expr(arg.clone(), ctx)?;
            if !self.unify(type_arg.ty(), signature.parameters[i], ctx) {
                return Err(Error::FunctionParameterMismatch {
                    span: ctx.program.convert_span(&call_node.span),
                    expected: ctx.get_type_detail(signature.parameters[i]),
                    found: ctx.get_type_detail(type_arg.ty()),
                });
            }
            args.push(type_arg);
        }

        let checked_expr = CheckedExprNode::Call(CheckedCallNode {
            callee: self.program.exprs.alloc_item(callee),
            generic_parameters: generic_parameters,
            args: self.program.exprs.alloc_items(args),
            type_id: self.substitute_all(signature.return_type, ctx)?,
            span: call_node.span,
        });

        if ctx.symbols[ty].is_function()
            && ctx.symbols[ty].as_function().unwrap().qualifier.is_const
        {
            let value = self
                .evaluator
                .evaluate_expr(&self.program, &checked_expr, ctx);
            let value = if value.is_type() {
                let type_id = self.substitute_all(value.to_type(), ctx)?;
                let const_id = ctx.symbols[type_id].as_const().unwrap().value;
                ctx.symbols.get_constant(const_id)
            } else {
                value.to_felt()
            };

            return Ok(CheckedExprNode::Value(CheckedValueNode::U32(value)));
        }

        return Ok(checked_expr);
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
                return Err(Error::TypeMismatch {
                    span: ctx.program.convert_span(&call_node.span),
                    expected: ctx.get_type_detail(generic_param.clone()),
                    found: ctx.get_type_detail(generic_arg),
                });
            }
        }
        let mut args = Vec::new();
        let receiver = {
            let receiver = self.visit_expr(call_node.receiver, ctx)?;
            if let Some((&underlying_type_id, generic_args, _)) =
                ctx.symbols[receiver.ty()].as_generic_instance()
            {
                self.populate_generic_arguments(underlying_type_id, generic_args.to_vec(), ctx)?;
            }
            if !self.unify(receiver.ty(), f.parameters[0].2, ctx) {
                return Err(Error::FunctionParameterMismatch {
                    span: ctx.program.convert_span(&call_node.span),
                    expected: ctx.get_type_detail(f.parameters[0].2),
                    found: ctx.get_type_detail(receiver.ty()),
                });
            }

            self.program.exprs.alloc_item(receiver)
        };

        for (i, arg) in call_node.args.iter().enumerate() {
            let type_arg = self.visit_expr(arg.clone(), ctx)?;
            if !self.unify(type_arg.ty(), f.parameters[i + 1].2, ctx) {
                return Err(Error::FunctionParameterMismatch {
                    span: ctx.program.convert_span(&call_node.span),
                    expected: ctx.get_type_detail(f.parameters[i + 1].2),
                    found: ctx.get_type_detail(type_arg.ty()),
                });
            }
            args.push(type_arg);
        }

        let checked_expr = CheckedExprNode::MemberCall(CheckedMemberCallNode {
            callee: self.program.exprs.alloc_item(variable),
            receiver,
            generic_parameters: f.generic_parameters.clone(),
            args: self.program.exprs.alloc_items(args),
            type_id: self.substitute_all(f.return_type, ctx)?,
            span: call_node.span,
        });

        if f.qualifier.is_const {
            let value = self
                .evaluator
                .evaluate_expr(&self.program, &checked_expr, ctx);
            eprintln!("DEBUGPRINT[391]: lib.rs:799: value={:#?}", value);
            let value = if value.is_type() {
                let type_id = self.substitute_all(value.to_type(), ctx)?;
                let const_id = ctx.symbols[type_id].as_const().unwrap().value;
                ctx.symbols.get_constant(const_id)
            } else {
                value.to_felt()
            };

            return Ok(CheckedExprNode::Value(CheckedValueNode::U32(value)));
        }

        return Ok(checked_expr);
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
            .map(|e| (e.ty(), self.program.exprs.alloc_item(e)))
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
                value: self.program.exprs.alloc_item(src_expr),
                target_type,
                span: cast_node.span,
            }));
        } else {
            return Err(Error::TypeMismatch {
                span: ctx.program.convert_span(&cast_node.span),
                expected: ctx.get_type_detail(target_type),
                found: ctx.get_type_detail(src_type),
            });
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
            return Err(Error::TypeMismatch {
                span: ctx.program.convert_span(&if_expr_node.span),
                expected: ctx.get_type_detail(BOOL_TYPE),
                found: ctx.get_type_detail(checked_expr.ty()),
            });
        }

        let checked_block = self.visit_expr(if_expr_node.if_branch.body, ctx)?;
        let if_type = checked_block.as_block_expr().unwrap().type_id;
        let if_branch = CheckedCase {
            predicate: self.program.exprs.alloc_item(checked_expr),
            type_id: BOOL_TYPE,
            body: self.program.exprs.alloc_item(checked_block),
        };

        let mut elseif_branches = Vec::with_capacity(if_expr_node.elseif_branches.len());
        for branch in &if_expr_node.elseif_branches {
            let checked_expr = self.visit_expr(branch.predicate, ctx)?;
            if !self.unify(checked_expr.ty(), BOOL_TYPE, ctx) {
                return Err(Error::TypeMismatch {
                    span: ctx.program.convert_span(&branch.span),
                    expected: ctx.get_type_detail(BOOL_TYPE),
                    found: ctx.get_type_detail(checked_expr.ty()),
                });
            }
            let checked_block = self.visit_expr(branch.body, ctx)?;
            let else_if_type = checked_block.as_block_expr().unwrap().type_id;

            if !self.unify(else_if_type, if_type, ctx) {
                return Err(Error::TypeMismatch {
                    span: ctx.program.convert_span(&branch.span),
                    expected: ctx.get_type_detail(else_if_type),
                    found: ctx.get_type_detail(if_type),
                });
            }

            elseif_branches.push(CheckedCase {
                predicate: self.program.exprs.alloc_item(checked_expr).clone(),
                type_id: BOOL_TYPE,
                body: self.program.exprs.alloc_item(checked_block),
            });
        }

        let else_branch = if let Some(else_branch) = if_expr_node.else_branch {
            let checked_block = self.visit_expr(else_branch, ctx)?;
            let else_type = checked_block.as_block_expr().unwrap().type_id;

            if !self.unify(else_type, if_type, ctx) {
                return Err(Error::TypeMismatch {
                    span: ctx.program.convert_span(&if_expr_node.span),
                    expected: ctx.get_type_detail(if_type),
                    found: ctx.get_type_detail(else_type),
                });
            }

            Some(self.program.exprs.alloc_item(checked_block))
        } else {
            None
        };

        Ok(CheckedExprNode::IfExpr(CheckedIfExprNode {
            if_branch,
            elseif_branches,
            else_branch,
            type_id: self.substitute_all(if_type, ctx)?,
            span: if_expr_node.span,
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
            return Err(Error::TypeMismatch {
                span: ctx.program.convert_span(&while_node.span),
                expected: ctx.get_type_detail(BOOL_TYPE),
                found: ctx.get_type_detail(predicate.ty()),
            });
        }
        let checked_block = self.visit_expr(while_node.body, ctx)?;
        Ok(CheckedStmtNode::While(CheckedWhileNode {
            predicate: self.program.exprs.alloc_item(predicate),
            type_id: BOOL_TYPE,
            body: self.program.exprs.alloc_item(checked_block),
            span: while_node.span,
        }))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_assignment(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let assignment_node = ctx.statement(node).as_assignment().cloned().unwrap();
        let checked_rhs = self.visit_expr(assignment_node.value, ctx)?;
        let checked_lhs = self.visit_expr(assignment_node.target, ctx)?;

        let lhs_ty = checked_lhs.ty();

        if !self.unify(lhs_ty, checked_rhs.ty(), ctx) {
            return Err(Error::TypeMismatch {
                span: ctx.program.convert_span(&assignment_node.span),
                expected: ctx.get_type_detail(lhs_ty),
                found: ctx.get_type_detail(checked_rhs.ty()),
            });
        }

        Ok(CheckedStmtNode::Assignment(CheckedAssignmentNode {
            target: self.program.exprs.alloc_item(checked_lhs),
            operator: assignment_node.operator,
            value: self.program.exprs.alloc_item(checked_rhs),
            type_id: self.substitute_all(lhs_ty, ctx)?,
            span: assignment_node.span,
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
            return Err(Error::TypeMismatch {
                span: ctx.program.convert_span(&variable_node.span),
                expected: format!("{} for let", ctx.get_type_detail(lhs_ty)),
                found: ctx.get_type_detail(rhs_ty),
            });
        }
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        ctx.symbols
            .declare_variable(
                variable_node.name,
                CheckedVariable::new(rhs_ty, variable_node.qualifier, current_scope_id, None),
            )
            .ok_or(error::Error::VariableAlreadyDefined {
                span: ctx.program.convert_span(&variable_node.span),
                variable: ctx.ident(variable_node.name).to_string(),
            })?;
        let checked_variable = CheckedVariableNode {
            name: variable_node.name,
            ty: self.substitute_all(rhs_ty, ctx)?,
            qualifier: variable_node.qualifier,
            value: self.program.exprs.alloc_item(checked_expr),
            scope_id: current_scope_id,
            span: variable_node.span,
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
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        let parent_scope_id = ctx.symbols.parent_scope_id().unwrap();
        if ctx.symbols[current_scope_id].kind != ScopeKind::Block {
            return Err(Error::InvalidReturn {
                span: ctx.program.convert_span(&return_node.span),
                message: format!(
                    "Cannot return from {:?} scope",
                    ctx.symbols[current_scope_id].kind
                ),
            });
        }
        let valid_kinds = [
            ScopeKind::LambdaFunction,
            ScopeKind::Function,
            ScopeKind::ImplMethod,
            ScopeKind::TraitMethod,
            ScopeKind::LambdaFunction,
        ];
        if !valid_kinds.contains(&ctx.symbols[parent_scope_id].kind) {
            return Err(Error::InvalidReturn {
                span: ctx.program.convert_span(&return_node.span),
                message: format!(
                    "Cannot return from {:?} scope",
                    ctx.symbols[parent_scope_id].kind
                ),
            });
        }

        let ret = if let Some(expr) = return_node.expr_id {
            let expr = self.visit_expr(expr, ctx)?;
            Some(self.program.exprs.alloc_item(expr))
        } else {
            None
        };

        Ok(CheckedStmtNode::Return(CheckedReturnNode {
            ret,
            span: return_node.span,
        }))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_intrinsic_stmt(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::StmtResult, Self::Error> {
        let node = ctx.statement(node).as_intrinsic().cloned().unwrap();
        match node {
            IntrinsicStmtNode::Assert {
                left,
                message,
                span,
            } => {
                let checked_lhs = self.visit_expr(left, ctx)?;

                if !self.unify(checked_lhs.ty(), BOOL_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: ctx.program.convert_span(&span),
                        expected: ctx.get_type_detail(BOOL_TYPE),
                        found: ctx.get_type_detail(checked_lhs.ty()),
                    });
                }

                Ok(CheckedStmtNode::Intrinsic(
                    CheckedIntrinsicStmtNode::Assert {
                        left: self.program.exprs.alloc_item(checked_lhs),
                        message: message,
                        span: span,
                    },
                ))
            }
            IntrinsicStmtNode::AssertEq {
                left,
                right,
                message,
                span,
            } => {
                let checked_lhs = self.visit_expr(left, ctx)?;
                let checked_rhs = self.visit_expr(right, ctx)?;

                if !self.unify(checked_lhs.ty(), checked_rhs.ty(), ctx) {
                    return Err(Error::TypeMismatch {
                        span: ctx.program.convert_span(&span),
                        expected: ctx.get_type_detail(checked_lhs.ty()),
                        found: ctx.get_type_detail(checked_rhs.ty()),
                    });
                }

                Ok(CheckedStmtNode::Intrinsic(
                    CheckedIntrinsicStmtNode::AssertEq {
                        left: self.program.exprs.alloc_item(checked_lhs),
                        right: self.program.exprs.alloc_item(checked_rhs),
                        message: message,
                        span: span,
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

        let underlying_type_id = ctx.symbols.get_type_id(None, impl_node.ty.name()).unwrap();
        let implementor_scope = ctx.symbols[underlying_type_id].scope_id();
        ctx.symbols.enter_scope(implementor_scope);
        ctx.symbols.start_scope(ScopeKind::Impl);
        ctx.infcx.enter_context();

        let mut checked_generic_parameters = Vec::new();
        for &generic_parameter in &impl_node.generic_parameters {
            checked_generic_parameters.push(ctx.symbols.add_type_variable(generic_parameter)?);
        }

        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, underlying_type_id)?;
        ctx.symbols
            .add_type_id(None, IdentId::SELF, underlying_type_id)?;

        let mut methods = Vec::new();

        for (generic_parameter, generic_arg) in checked_generic_parameters
            .iter()
            .zip(ctx.symbols[underlying_type_id].generic_parameters())
        {
            if !self.unify(generic_parameter.clone(), generic_arg, ctx) {
                return Err(Error::TypeMismatch {
                    span: ctx.program.convert_span(&impl_node.span),
                    expected: ctx.get_type_detail(generic_parameter.clone()),
                    found: ctx.get_type_detail(generic_arg),
                });
            }
        }

        for &function_id in &impl_node.body {
            methods.push(CheckedDefinitionNode::Function(
                self.typecheck_method(function_id, ctx)?,
            ));
        }

        let checked_impl = CheckedImplNode {
            generic_parameters: checked_generic_parameters,
            ty: underlying_type_id,
            body: self.program.defs.alloc_items(methods),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
        };
        // let ty = Type::Impl(checked_impl.clone());
        // symbols.add_type(symbols.parent_scope_id(), ty);

        ctx.infcx.exit_context();
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
        ctx.infcx.enter_context();
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
            body: self.program.defs.alloc_items(methods),
            unchecked_body: trait_node.body.clone(),
            implementors: Vec::new(),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            visibility: trait_node.visibility,
            span: trait_node.span,
        };
        // TODO: remove clone
        let ty = Type::Trait(checked_trait.clone());
        ctx.symbols
            .add_type(ctx.symbols.parent_scope_id(), checked_trait.name, ty)?;

        ctx.infcx.exit_context();
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
        ctx.infcx.enter_context();
        let checked_function = self.typecheck_function(node, ctx)?;
        let ty = Type::Function(checked_function.clone());
        ctx.symbols
            .add_type(ctx.symbols.parent_scope_id(), ty.name(), ty)?;
        ctx.infcx.exit_context();
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
        ctx.infcx.enter_context();
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
            span: struct_node.span,
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

        ctx.infcx.exit_context();
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
        ctx.infcx.enter_context();
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
            span: enum_node.span,
        };
        let ty = Type::Enum(checked_enum.clone());
        ctx.symbols
            .add_type(ctx.symbols.parent_scope_id(), checked_enum.name, ty)?;

        ctx.infcx.exit_context();
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
        ctx.infcx.enter_scope();
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
            NodeType::MatchExpr => self.visit_match(expr_id, ctx)?,
            _ => std::unreachable!(),
        };
        ctx.infcx.exit_scope();
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
                self.program.defs.alloc_item(definition)
            }),
            NodeType::ExpressionStmt => Self::StmtResult::from({
                let expr =
                    self.visit_expr(ctx.statement(stmt_id).as_expression().unwrap().clone(), ctx)?;
                self.program.exprs.alloc_item(expr)
            }),
            NodeType::IntrinsicStmt => self.visit_intrinsic_stmt(stmt_id, ctx)?,
            _ => unreachable!(),
        };
        ctx.pop_node_id();
        Ok(res)
    }

    #[instrument(level = "debug", skip_all)]
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
                self.visit_module(module_id, ctx).unwrap_or_else(|err| {
                    let report = crate::error::lowering_error_to_report(err);
                    report
                        .eprint(ariadne::FnCache::new(|x: &String| {
                            Ok(std::fs::read_to_string(std::path::Path::new(x.as_str())).unwrap())
                        }))
                        .unwrap();
                    std::process::exit(1);
                });
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
        // TODO: remove clone
        let BlockExprNode { stmts, expr, span } =
            ctx.expression(node).as_block_expr().unwrap().clone();
        ctx.symbols.start_scope(ScopeKind::Block);

        let current_scope_id = ctx.symbols.current_scope_id().unwrap();

        let mut checked_stmts = Vec::with_capacity(stmts.len());

        for (i, stmt) in stmts.iter().enumerate() {
            let checked_stmt = self.visit_stmt(stmt.clone(), ctx)?;
            if checked_stmt.is_return() && (i != stmts.len() - 1 || expr.is_some()) {
                return Err(Error::InvalidReturn {
                    span: ctx
                        .program
                        .convert_span(&checked_stmt.as_return().unwrap().span),
                    message: format!("Quick return"),
                });
            }
            checked_stmts.push(checked_stmt);
        }

        let (return_type_id, checked_return_expr) = match expr {
            Some(expr) => {
                let checked_expr = self.visit_expr(expr, ctx)?;
                (checked_expr.ty(), Some(checked_expr))
            }
            None => {
                if let Some(CheckedReturnNode {
                    ret: Some(ret),
                    span: _span,
                }) = checked_stmts.last().and_then(|x| x.as_return())
                {
                    (self.program.exprs[ret.clone()].ty(), None)
                } else {
                    (VOID_TYPE, None)
                }
            }
        };

        ctx.symbols.end_scope();

        let checked_block_expr = CheckedBlockExprNode {
            stmts: self.program.stmts.alloc_items(checked_stmts),
            expr: checked_return_expr.map(|e| self.program.exprs.alloc_item(e)),
            type_id: return_type_id,
            scope_id: current_scope_id,
            span: span,
        };

        Ok(CheckedExprNode::BlockExpr(checked_block_expr))
    }

    #[instrument(level = "debug", skip_all)]
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
            return Err(Error::TypeMismatch {
                span: ctx.program.convert_span(&node.span),
                expected: ctx.get_type_detail(rhs_ty),
                found: ctx.get_type_detail(lhs_ty),
            });
        }

        let value = self
            .evaluator
            .evaluate_expr(&self.program, &value, ctx)
            .to_value();

        let node = CheckedConstNode {
            name: Some(node.name),
            ty: rhs_ty,
            value: ctx.symbols.add_constant(value),
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
        let start = self.visit_expr(for_node.start, ctx)?;
        let end = self.visit_expr(for_node.end, ctx)?;
        if !(self.unify(start.ty(), FELT_TYPE, ctx) && self.unify(end.ty(), FELT_TYPE, ctx)
            || self.unify(start.ty(), U32_TYPE, ctx) && self.unify(end.ty(), U32_TYPE, ctx))
        {
            return Err(Error::TypeMismatch {
                span: ctx.program.convert_span(&for_node.span),
                expected: format!("Felt or U32 for for loop variable"),
                found: ctx.get_type_detail(start.ty()),
            });
        }

        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        let variable =
            CheckedVariable::new(start.ty(), TypeQualifier::new(true), current_scope_id, None);
        ctx.symbols
            .declare_variable(for_node.variable, variable)
            .ok_or(error::Error::VariableAlreadyDefined {
                span: ctx.program.convert_span(&for_node.span),
                variable: ctx.ident(for_node.variable).to_string(),
            })?;

        ctx.symbols.start_scope(ScopeKind::Block);
        let checked_block = self.visit_expr(for_node.body, ctx)?;
        let node = CheckedStmtNode::For(CheckedForNode {
            variable: for_node.variable,
            start: self.program.exprs.alloc_item(start),
            end: self.program.exprs.alloc_item(end),
            body: self.program.exprs.alloc_item(checked_block),
            scope_id: current_scope_id,
            span: for_node.span,
        });
        ctx.symbols.end_scope();
        ctx.symbols.end_scope();
        Ok(node)
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_match(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> std::result::Result<Self::ExprResult, Self::Error> {
        //get match node
        let match_node = ctx.expression(node).as_match().cloned().unwrap();
        let checked_scrutinee = self.visit_expr(match_node.scrutinee, ctx)?;

        //There are two type constraints here, one is that the scrutinee_type must be consistent with the type of the value of the pattern
        //The other is that the return types of all cases must be consistent
        let scrutinee_type = checked_scrutinee.ty();

        let mut checked_arms = Vec::new();
        let mut match_expr_type: Option<TypeId> = None;
        let mut wildcard_case: Option<CheckedMatchArm> = None;

        for (_idx, arm) in match_node.arms.iter().enumerate() {
            let checked_pattern = match &arm.pattern {
                MatchPattern::Value(pattern_expr) => {
                    let checked_pattern_expr = self.visit_expr(*pattern_expr, ctx)?;
                    let pattern_type = checked_pattern_expr.ty();

                    if !self.unify(scrutinee_type, pattern_type, ctx) {
                        return Err(Error::TypeMismatch {
                            span: ctx.program.convert_span(&match_node.span),
                            expected: ctx.get_type_detail(scrutinee_type),
                            found: ctx.get_type_detail(pattern_type),
                        });
                    }
                    Some(self.program.exprs.alloc_item(checked_pattern_expr))
                }
                MatchPattern::PlaceHolder => {
                    if wildcard_case.is_some() {
                        return Err(Error::DuplicateWildcard);
                    }
                    None
                }
            };

            let checked_body = self.visit_expr(arm.body, ctx)?;
            let arm_body_type = checked_body.ty();

            match_expr_type.get_or_insert(arm_body_type);
            if !self.unify(match_expr_type.unwrap(), arm_body_type, ctx) {
                return Err(Error::TypeMismatch {
                    span: ctx.program.convert_span(&match_node.span),
                    expected: ctx.get_type_detail(match_expr_type.unwrap()),
                    found: ctx.get_type_detail(arm_body_type),
                });
            }

            let checked_arm = CheckedMatchArm {
                pattern: checked_pattern,
                body: self.program.exprs.alloc_item(checked_body),
            };

            //note: move the wildcard case to the end
            if checked_arm.pattern.is_none() {
                wildcard_case = Some(checked_arm);
            } else {
                checked_arms.push(checked_arm);
            }
        }

        if let Some(placeholder) = wildcard_case {
            checked_arms.push(placeholder);
        }

        Ok(CheckedExprNode::Match(CheckedMatchNode {
            value: self.program.exprs.alloc_item(checked_scrutinee),
            cases: checked_arms,
            type_id: match_expr_type.unwrap_or(VOID_TYPE),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
        }))
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
            ctx.symbols
                .declare_variable(parameter.clone(), variable)
                .ok_or(error::Error::VariableAlreadyDefined {
                    span: ctx.program.convert_span(&function.span),
                    variable: ctx.ident(parameter.clone()).to_string(),
                })?;
            parameters.push((parameter.clone(), *mutable, parameter_type));
        }

        let expected_return_type = if let Some(ref ret) = function.return_type {
            self.typecheck(ret, ctx)?
        } else {
            VOID_TYPE
        };

        let checked_body = {
            let checked_body = self.visit_expr(function.body.clone(), ctx)?;
            let actual_return_type = checked_body.ty();
            if !self.unify(expected_return_type, actual_return_type, ctx) {
                return Err(Error::TypeMismatch {
                    span: ctx.program.convert_span(&function.span),
                    expected: format!(
                        "{} for return value",
                        ctx.get_type_detail(expected_return_type)
                    ),
                    found: ctx.get_type_detail(actual_return_type),
                });
            }
            checked_body
        };

        let mut checked_function = CheckedLambdaFunctionNode {
            name: ctx.intern_lambda(),
            parameters,
            body: self.program.exprs.alloc_item(checked_body),
            return_type: expected_return_type,
            scope_id: current_scope_id,
            type_id: UNKOWN_TYPE,
            span: function.span,
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
        ctx.infcx.enter_context();

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
        let mut unimplemented_methods: HashSet<DefId> =
            trait_node.unchecked_body.iter().cloned().collect();
        let mut checked_methods = Vec::with_capacity(trait_node.body.len());

        for &generic_parameter in &impl_node.generic_parameters {
            let type_id = ctx.symbols.add_type_variable(generic_parameter)?;
            generic_parameters.push(type_id);
        }

        for &function_id in &impl_node.body {
            let method = self.typecheck_method(function_id, ctx)?;
            let i = trait_node
                .body
                .iter()
                .position(|&trait_def_id| {
                    let trait_function = self.program.defs[trait_def_id].as_function().unwrap();
                    trait_function.trait_impl_signature(implementor_type_id) == method.signature()
                        && trait_function.name == method.name
                })
                .ok_or(Error::UnresolvedTraitMethod {
                    method_span: ctx.program.convert_span(&method.span),
                    trait_name: format!("{}", ctx.ident(trait_node.name)),
                    method_name: format!("{}", ctx.ident(method.name)),
                })?;
            unimplemented_methods.remove(&trait_node.unchecked_body[i]);
            checked_methods.push(CheckedDefinitionNode::Function(method));
        }

        for unimplemented_method in unimplemented_methods {
            let method = self.typecheck_method(unimplemented_method, ctx)?;
            assert!(method.body.is_some());
            checked_methods.push(CheckedDefinitionNode::Function(method));
        }

        let checked_impl = CheckedImplTraitNode {
            generic_parameters,
            trait_ty: trait_type_id,
            ty: implementor_type_id,
            body: self.program.defs.alloc_items(checked_methods),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
        };
        // let ty = Type::Impl(checked_impl.clone());
        // symbols.add_type(symbols.parent_scope_id(), ty);

        ctx.symbols
            .impl_trait_for_type(trait_type_id, implementor_type_id);

        ctx.infcx.exit_context();
        ctx.symbols.end_scope();
        ctx.symbols.exit_scope();
        Ok(CheckedDefinitionNode::ImplTrait(checked_impl))
    }
}

impl<F: Clone + From<u32> + ContextFelt, C> TypeChecker<F, C> {
    pub fn new(program: CheckedProgram<F>, evaluator: Box<dyn Evaluator<F, C>>) -> Self {
        Self {
            program,
            evaluator,
            _marker: std::marker::PhantomData,
        }
    }

    // NOTE: primitive???
    #[instrument(level = "debug", skip_all)]
    fn typecheck_std_primitive_module(
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
        self.typecheck_array(ctx)?;
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_member_access(
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
    fn populate_generic_arguments(
        &mut self,
        underlying_type_id: TypeId,
        generic_args: Vec<TypeId>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        for (generic_param, generic_arg) in ctx.symbols[underlying_type_id]
            .generic_parameters()
            .into_iter()
            .zip(generic_args)
        {
            if !self.unify(generic_param, generic_arg, ctx) {
                return Err(Error::TypeMismatch {
                    span: ctx.program.convert_span(&Default::default()),
                    expected: ctx.get_type_detail(generic_param),
                    found: ctx.get_type_detail(generic_arg),
                });
            }
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    fn populate_constant_u32(
        &mut self,
        value: usize,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        let value = self
            .evaluator
            .from_constant_u32(u32::try_from(value).unwrap());
        let node = CheckedConstNode {
            name: None,
            ty: U32_TYPE,
            value: ctx.symbols.add_constant(value),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            visibility: Visibility::Public,
        };

        ctx.symbols
            .get_or_add_type(None, TypeKey::from_const_id(node.value), Type::Const(node))
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_method_receiver(
        &mut self,
        function: &FunctionNode,
        ctx: &TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        let is_method = [ScopeKind::ImplMethod, ScopeKind::TraitMethod]
            .contains(&ctx.symbols[current_scope_id].kind);

        for (i, (parameter_name, _, parameter_type)) in function.parameters.iter().enumerate() {
            if parameter_name == &IdentId::SELF && (i != 0 || !is_method) {
                return Err(Error::InvalidSelfParameter {
                    span: ctx.program.convert_span(&function.span),
                    message: format!("Invalid self parameter at {}", i),
                });
            }

            if parameter_type == &UncheckedType::Basic(IdentId::TYPE_SELF) && !is_method {
                return Err(Error::InvalidSelfParameter {
                    span: ctx.program.convert_span(&function.span),
                    message: format!("Invalid Self parameter at {}", i),
                });
            }
        }

        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_array(&mut self, ctx: &mut TypeCheckerVisitorContext<F, C>) -> Result<TypeId> {
        ctx.symbols.start_scope(ScopeKind::Array);
        ctx.infcx.enter_context();

        let inner_ty = ctx.symbols.add_type_variable(IdentId::T)?;
        let size = ctx.symbols.add_type_variable(IdentId::N)?;

        let checked_array = CheckedArrayNode {
            inner_ty,
            size_ty: size,
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            implementations: vec![],
        };

        let ty = Type::Array(checked_array.clone());
        let type_id = ctx
            .symbols
            .add_type(ctx.symbols.parent_scope_id(), ty.name(), ty)?;

        ctx.infcx.exit_context();
        ctx.symbols.end_scope();
        Ok(type_id)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck(
        &mut self,
        ty: &UncheckedType,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        match ty {
            UncheckedType::Basic(IdentId::TYPE_BOOL) => Ok(BOOL_TYPE),
            UncheckedType::Basic(IdentId::TYPE_FELT) => Ok(FELT_TYPE),
            UncheckedType::Basic(IdentId::TYPE_U32) => Ok(U32_TYPE),
            UncheckedType::Basic(name) => {
                Ok(ctx
                    .symbols
                    .get_type_id(None, name.clone())
                    .ok_or(Error::UnresolvedType {
                        span: ctx.program.convert_span(&Default::default()),
                        resolved_type: format!("{}", ctx.ident(name.clone())),
                    })?)
            }
            UncheckedType::Generic(name, generic_parameters) => {
                let underlying_type_id =
                    ctx.symbols
                        .get_type_id(None, name.clone())
                        .ok_or(Error::UnresolvedType {
                            span: ctx.program.convert_span(&Default::default()),
                            resolved_type: format!("{}", ctx.ident(name.clone())),
                        })?;

                let mut checked_generic_parameters = Vec::new();
                for generic_parameter in generic_parameters {
                    checked_generic_parameters.push(self.typecheck(generic_parameter, ctx)?);
                }

                match &ctx.symbols[underlying_type_id] {
                    Type::Struct(checked_struct) => {
                        if checked_struct.generic_parameters.len()
                            != checked_generic_parameters.len()
                        {
                            return Err(Error::GenericParameterMismatch {
                                span: ctx.program.convert_span(&checked_struct.span),
                                expected: format!(
                                    "{} generic parameters",
                                    checked_struct.generic_parameters.len()
                                ),
                                found: format!("{}", checked_generic_parameters.len()),
                            });
                        }

                        let ty = Type::GenericInstance(
                            underlying_type_id,
                            checked_generic_parameters,
                            ctx.symbols.current_scope_id().unwrap(),
                        );
                        let type_id = ctx.symbols.get_or_add_type(None, ty.key(), ty)?;

                        Ok(type_id)
                    }
                    Type::Enum(checked_enum) => {
                        if checked_enum.generic_parameters.len() != checked_generic_parameters.len()
                        {
                            return Err(Error::GenericParameterMismatch {
                                span: ctx.program.convert_span(&checked_enum.span),
                                expected: format!(
                                    "{} generic parameters",
                                    checked_enum.generic_parameters.len()
                                ),
                                found: format!("{}", checked_generic_parameters.len()),
                            });
                        }

                        todo!()
                    }
                    Type::Function(checked_function) => {
                        if checked_function.generic_parameters.len()
                            != checked_generic_parameters.len()
                        {
                            return Err(Error::GenericParameterMismatch {
                                span: ctx.program.convert_span(&checked_function.span),
                                expected: format!(
                                    "{} generic parameters",
                                    checked_function.generic_parameters.len()
                                ),
                                found: format!("{}", checked_generic_parameters.len()),
                            });
                        }

                        todo!()
                    }
                    Type::Trait(checked_trait) => {
                        if checked_trait.generic_parameters.len()
                            != checked_generic_parameters.len()
                        {
                            return Err(Error::GenericParameterMismatch {
                                span: ctx.program.convert_span(&checked_trait.span),
                                expected: format!(
                                    "{} generic parameters",
                                    checked_trait.generic_parameters.len()
                                ),
                                found: format!("{}", checked_generic_parameters.len()),
                            });
                        }
                        todo!()
                    }
                    Type::Array(_checked_array) => {
                        if checked_generic_parameters.len() != 2 {
                            return Err(Error::GenericParameterMismatch {
                                span: Default::default(),
                                expected: format!("2 generic parameters for array"),
                                found: format!("{}", checked_generic_parameters.len()),
                            });
                        }

                        let ty = Type::GenericInstance(
                            underlying_type_id,
                            checked_generic_parameters,
                            ctx.symbols.current_scope_id().unwrap(),
                        );
                        let type_id = ctx.symbols.get_or_add_type(None, ty.key(), ty)?;

                        Ok(type_id)
                    }
                    _ => unreachable!(),
                }
            }
            UncheckedType::Array(inner, size) => {
                let underlying_type_id = ctx
                    .symbols
                    .get_type_id(Some(ScopeId::primitive()), IdentId::TYPE_ARRAY)
                    .unwrap();
                let inner_ty = self.typecheck(inner, ctx)?;

                let size_ty = self.populate_constant_u32(size.clone(), ctx)?;

                let ty = Type::GenericInstance(
                    underlying_type_id,
                    vec![inner_ty, size_ty],
                    ctx.symbols.current_scope_id().unwrap(),
                );
                let type_id = ctx.symbols.get_or_add_type(None, ty.key(), ty)?;

                Ok(type_id)
            }
            UncheckedType::Tuple(elements) => {
                // check each element and collect results into a Result<Vec<TypeId>>
                let checked_elements = elements
                    .iter()
                    .map(|elem_ty| self.typecheck(elem_ty, ctx))
                    .collect::<Result<_>>()?;

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
        ctx.infcx.enter_scope();

        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, UNKOWN_TYPE)?;

        let checked_function = self.typecheck_function(function_id, ctx)?;
        let ty = Type::Function(checked_function.clone());
        ctx.symbols.add_type(None, checked_function.name, ty)?;

        ctx.infcx.exit_scope();
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
        ctx.infcx.enter_scope();

        let checked_function = self.typecheck_function(function_id, ctx)?;
        let ty = Type::Function(checked_function.clone());
        ctx.symbols.add_type(None, checked_function.name, ty)?;

        ctx.infcx.exit_scope();
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
            ctx.symbols
                .declare_variable(parameter.clone(), variable)
                .ok_or(error::Error::VariableAlreadyDefined {
                    span: ctx.program.convert_span(&function.span),
                    variable: format!("{}", ctx.ident(parameter.clone())),
                })?;
            parameters.push((parameter.clone(), *mutable, parameter_type));
        }

        let expected_return_type = if let Some(ref ret) = function.return_type {
            self.typecheck(ret, ctx)?
        } else {
            VOID_TYPE
        };

        let checked_body = if let Some(body) = &function.body {
            let checked_body = self.visit_expr(body.clone(), ctx)?;
            let actual_return_type = checked_body.ty();

            if !self.unify(expected_return_type, actual_return_type, ctx) {
                return Err(Error::TypeMismatch {
                    span: ctx.program.convert_span(&function.span),
                    expected: format!(
                        "{} for return value",
                        ctx.get_type_detail(expected_return_type)
                    ),
                    found: ctx.get_type_detail(actual_return_type),
                });
            }
            Some(checked_body)
        } else {
            None
        };

        let checked_function = CheckedFunctionNode {
            name: function.name,
            parameters,
            qualifier: function.qualifier,
            generic_parameters,
            body: checked_body.map(|x| self.program.exprs.alloc_item(x)),
            return_type: expected_return_type,
            scope_id: current_scope_id,
            visibility: function.visibility,
            attrs: function.attrs,
            span: function.span,
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
                ctx.infcx.equate(lhs_ty, rhs_ty);
                true
            }
            (_, Type::TypeVariable(_)) => {
                ctx.infcx.equate(rhs_ty, lhs_ty);
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
                // TODO: remove clone
                let a1 = a1.clone();
                let a2 = a2.clone();
                self.unify(a1.inner_ty, a2.inner_ty, ctx) && self.unify(a1.size_ty, a2.size_ty, ctx)
            }
            (Type::Tuple(t1), Type::Tuple(t2)) => {
                for (lhs_ty, rhs_ty) in t1.clone().into_iter().zip(t2.clone().into_iter()) {
                    if !self.unify(lhs_ty, rhs_ty, ctx) {
                        return false;
                    }
                }
                true
            }

            (Type::GenericInstance(underlying_type_id, args, _), Type::Struct(checked_struct))
            | (Type::Struct(checked_struct), Type::GenericInstance(underlying_type_id, args, _)) => {
                let other_ty = &ctx.symbols[*underlying_type_id];
                other_ty.is_struct()
                    && other_ty.name() == checked_struct.name
                    && other_ty.scope_id() == checked_struct.scope_id
                    && args
                        .clone()
                        .into_iter()
                        .zip(checked_struct.generic_parameters.clone().into_iter())
                        .all(|(p1, p2)| self.unify(p1, p2, ctx))
            }
            (Type::GenericInstance(underlying_type_id, args, _), Type::Array(checked_array))
            | (Type::Array(checked_array), Type::GenericInstance(underlying_type_id, args, _)) => {
                let other_ty = &ctx.symbols[*underlying_type_id];
                other_ty.is_array()
                    && args
                        .clone()
                        .into_iter()
                        .zip(vec![checked_array.inner_ty, checked_array.size_ty])
                        .all(|(p1, p2)| self.unify(p1, p2, ctx))
            }

            (
                Type::GenericInstance(lhs_ty, lhs_args, _),
                Type::GenericInstance(rhs_ty, rhs_args, _),
            ) => {
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
            (Type::Const(c), Type::Const(d)) => c.ty == d.ty,
            (Type::Const(c), _) => c.ty == rhs_ty,
            (_, Type::Const(c)) => c.ty == lhs_ty,
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
        if !ctx.infcx.has_equations() {
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
        if let Some(subst_type) = ctx.infcx.probe(type_id) {
            return Ok(subst_type);
        }

        match ctx.symbols[type_id].clone() {
            Type::TypeVariable(_) => Ok(type_id),

            Type::Array(array) => {
                let ty = Type::Array(CheckedArrayNode {
                    inner_ty: self.substitute_all(array.inner_ty, ctx)?,
                    size_ty: self.substitute_all(array.size_ty, ctx)?,
                    scope_id: array.scope_id,
                    implementations: array.implementations,
                });
                ctx.symbols
                    .get_or_add_type(Some(ScopeId::primitive()), ty.key(), ty)
            }

            Type::GenericInstance(underlying_type_id, generic_parameters, scope_id) => {
                let mut new_generic_parameters = Vec::new();
                for generic_parameter in generic_parameters {
                    new_generic_parameters.push(self.substitute_all(generic_parameter, ctx)?);
                }

                let ty =
                    Type::GenericInstance(underlying_type_id, new_generic_parameters, scope_id);
                ctx.symbols.get_or_add_type(None, ty.key(), ty)
            }

            _ => Ok(type_id),
        }
    }
}
