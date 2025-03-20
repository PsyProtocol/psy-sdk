#![feature(if_let_guard)]

mod constraint;
mod context;
mod definition;
mod expr;
mod generic;
mod implementer;
mod infer;
mod program;
mod resolver;
mod rewriter;
mod stmt;
mod symbol_table;
mod traits;
mod r#type;
mod value;
mod variable;

mod error;
mod visualizer;

pub use constraint::*;
pub use context::*;
pub use definition::*;
pub use error::*;
pub use expr::*;
pub use generic::*;
pub use implementer::*;
pub use infer::*;
pub use program::*;
pub use r#type::*;
pub use resolver::*;
pub use stmt::*;
pub use symbol_table::*;
pub use traits::*;
pub use value::*;
pub use variable::*;
pub use visualizer::*;

use anyhow::anyhow;
use qed_ast::*;
use std::collections::{HashMap, HashSet};
use std::result::Result as StdResult;
use tracing::instrument;

use indexmap::IndexMap;
use itertools::Itertools;
use qed_common::FileId;
use qedlang_core::dpn::ops::context_trait::ContextFelt;

pub struct TypeChecker<F: Clone + From<u32> + ContextFelt, C> {
    pub program: CheckedProgram<F>,
    evaluator: Box<dyn Evaluator<F, C>>,
    pub resolver: ResolverCtxt,
    pub infcx: InferCtxt<F, C>,
    pub implementer: ImplementerCtxt,

    _marker: std::marker::PhantomData<C>,
}

impl<F: Clone + From<u32> + ContextFelt, C> AstVisitor<F, C> for TypeChecker<F, C> {
    type Expr = ExprNode<F>;

    type Stmt = StmtNode;

    type Definition = DefinitionNode;

    type ExprResult = CheckedExprNode<F>;

    type StmtResult = CheckedStmtNode;

    type DefinitionResult = DefId;

    type Context = TypeCheckerVisitorContext<F, C>;

    type Error = Error;

    #[instrument(level = "debug", skip_all)]
    fn visit_use(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let node = ctx.definition(def_id).as_use().cloned().unwrap();
        self.add_use(&node, ctx)?;
        Ok(self
            .program
            .defs
            .alloc_item(CheckedDefinitionNode::Use(node)))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_path(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let path_node = ctx.expression(node).as_path().cloned().unwrap();
        return Ok(CheckedExprNode::Path(self.resolve_path(&path_node, ctx)?));
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_index_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let index_access_node = ctx.expression(node).as_index_access().cloned().unwrap();
        let checked_expr = self.visit_expr(index_access_node.target, ctx)?;
        let checked_index = self.visit_expr(index_access_node.index, ctx)?;
        let type_id = checked_expr.ty();
        let ty = &ctx.symbols[type_id];

        let inner_ty = ty.as_array().unwrap().inner_ty;
        if !self.unify(checked_index.ty(), FELT_TYPE, ctx) {
            return Err(Error::TypeMismatch {
                span: checked_index.span(),
                expected: vec![FELT_TYPE],
                found: checked_index.ty(),
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
    ) -> StdResult<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let member_access_node = ctx.expression(node).as_member_access().cloned().unwrap();
        let checked_expr = self.visit_expr(member_access_node.target, ctx)?;
        let type_id = checked_expr.ty();

        if ctx.ancestor_node_type(1).is_member_call_expr() {
            let (_, type_id) = self.find_impl(type_id, member_access_node.field, ctx)?;
            let visibility = ctx.symbols[type_id].visibility();
            if !(visibility.is_public()
                || self.typecheck_member_access(member_access_node.target, ctx))
            {
                return Err(Error::MemberNotPublic {
                    span: member_access_node.span,
                    ty: type_id,
                    field: member_access_node.field,
                });
            }
            return Ok(CheckedExprNode::MemberAccess(CheckedMemberAccessNode {
                target: self.program.exprs.alloc_item(checked_expr),
                field: member_access_node.field,
                type_id,
                span: member_access_node.span,
            }));
        } else {
            let fields = &ctx.symbols[type_id].as_struct().unwrap().fields;
            let CheckedStructField {
                ty: field_type,
                visibility,
                ..
            } = fields
                .get(&member_access_node.field)
                .ok_or(Error::UnresolvedMember {
                    span: member_access_node.span,
                    member_name: member_access_node.field,
                })?;
            if !(visibility.is_public()
                || self.typecheck_member_access(member_access_node.target, ctx))
            {
                return Err(Error::MemberNotPublic {
                    span: member_access_node.span,
                    ty: type_id,
                    field: member_access_node.field,
                });
            }
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
    ) -> StdResult<Self::ExprResult, Self::Error> {
        // get TupleAccessNode
        let tuple_access_node = ctx.expression(node).as_tuple_access().cloned().unwrap();

        let checked_expr = self.visit_expr(tuple_access_node.target, ctx)?;
        let type_id = checked_expr.ty();
        let ty = &ctx.symbols[type_id];
        let element_types = ty.as_tuple().ok_or(anyhow!("Expected tuple type"))?;

        if tuple_access_node.index >= element_types.len() {
            return Err(Error::IndexOutOfBounds {
                span: tuple_access_node.span,
                index: tuple_access_node.index,
                length: element_types.len(),
            });
        }

        let field_type = element_types
            .get(tuple_access_node.index)
            .ok_or(Error::IndexOutOfBounds {
                span: tuple_access_node.span,
                index: tuple_access_node.index,
                length: element_types.len(),
            })?
            .clone();
        Ok(CheckedExprNode::TupleAccess(CheckedTupleAccessNode {
            target: self.program.exprs.alloc_item(checked_expr),
            index: tuple_access_node.index,
            type_id: self.substitute_all(field_type, ctx)?,
            span: tuple_access_node.span,
        }))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_intrinsic_expr(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let intrinsic_node = ctx.expression(node).as_intrinsic().cloned().unwrap();
        match intrinsic_node {
            IntrinsicExprNode::GetUserId { span } => {
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetUserId {
                        type_id: FELT_TYPE,
                        span,
                    },
                ));
            }
            IntrinsicExprNode::GetContractId { span } => {
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetContractId {
                        type_id: FELT_TYPE,
                        span,
                    },
                ));
            }
            IntrinsicExprNode::GetCheckpointId { span } => {
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetCheckpointId {
                        type_id: FELT_TYPE,
                        span,
                    },
                ));
            }
            IntrinsicExprNode::GetLastNonce { span } => {
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetLastNonce {
                        type_id: FELT_TYPE,
                        span,
                    },
                ));
            }
            IntrinsicExprNode::GetUserPublicKeyHash { span } => {
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetUserPublicKeyHash {
                        type_id: HASH_TYPE,
                        span,
                    },
                ));
            }
            IntrinsicExprNode::GetStateHashAt { slot_index, span } => {
                let slot_index = self.visit_expr(slot_index, ctx)?;
                if !self.unify(slot_index.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: slot_index.span(),
                        expected: vec![FELT_TYPE],
                        found: slot_index.ty(),
                    });
                }
                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::GetStateHashAt {
                        slot_index: self.program.exprs.alloc_item(slot_index),
                        type_id: HASH_TYPE,
                        span,
                    },
                ));
            }
            IntrinsicExprNode::GetOtherContractStateHashAt {
                contract_state_tree_height,
                contract_id,
                slot_index,
                span,
            } => {
                let contract_state_tree_height =
                    self.visit_expr(contract_state_tree_height, ctx)?;
                let contract_id = self.visit_expr(contract_id, ctx)?;
                let slot_index = self.visit_expr(slot_index, ctx)?;
                if !self.unify(contract_state_tree_height.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: contract_state_tree_height.span(),
                        expected: vec![FELT_TYPE],
                        found: contract_state_tree_height.ty(),
                    });
                }
                if !self.unify(contract_id.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: contract_id.span(),
                        expected: vec![FELT_TYPE],
                        found: contract_id.ty(),
                    });
                }
                if !self.unify(slot_index.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: slot_index.span(),
                        expected: vec![FELT_TYPE],
                        found: slot_index.ty(),
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
                        span,
                    },
                ));
            }
            IntrinsicExprNode::GetOtherUserContractStateHashAt {
                contract_state_tree_height,
                user_id,
                contract_id,
                slot_index,
                span,
            } => {
                let contract_state_tree_height =
                    self.visit_expr(contract_state_tree_height, ctx)?;
                let user_id = self.visit_expr(user_id, ctx)?;
                let contract_id = self.visit_expr(contract_id, ctx)?;
                let slot_index = self.visit_expr(slot_index, ctx)?;
                if !self.unify(contract_state_tree_height.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: contract_state_tree_height.span(),
                        expected: vec![FELT_TYPE],
                        found: contract_state_tree_height.ty(),
                    });
                }
                if !self.unify(user_id.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: user_id.span(),
                        expected: vec![FELT_TYPE],
                        found: user_id.ty(),
                    });
                }
                if !self.unify(contract_id.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: contract_id.span(),
                        expected: vec![FELT_TYPE],
                        found: contract_id.ty(),
                    });
                }
                if !self.unify(slot_index.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: slot_index.span(),
                        expected: vec![FELT_TYPE],
                        found: slot_index.ty(),
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
                        span,
                    },
                ));
            }
            IntrinsicExprNode::CSetStateHashAt {
                slot_index,
                new_value,
                span,
            } => {
                let slot_index = self.visit_expr(slot_index, ctx)?;
                let new_value = self.visit_expr(new_value, ctx)?;

                if !self.unify(slot_index.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: slot_index.span(),
                        expected: vec![FELT_TYPE],
                        found: slot_index.ty(),
                    });
                }
                if !self.unify(new_value.ty(), HASH_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: new_value.span(),
                        expected: vec![HASH_TYPE],
                        found: new_value.ty(),
                    });
                }

                return Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::CSetStateHashAt {
                        slot_index: self.program.exprs.alloc_item(slot_index),
                        new_value: self.program.exprs.alloc_item(new_value),
                        type_id: HASH_TYPE,
                        span,
                    },
                ));
            }
            IntrinsicExprNode::Read { offset, span } => {
                // TODO: remove clone
                let offset = self.visit_expr(offset, ctx)?;
                if !self.unify(offset.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: offset.span(),
                        expected: vec![FELT_TYPE],
                        found: offset.ty(),
                    });
                }
                return Ok(CheckedExprNode::Intrinsic(CheckedIntrinsicExprNode::Read {
                    offset: self.program.exprs.alloc_item(offset),
                    type_id: FELT_TYPE,
                    span,
                }));
            }
            IntrinsicExprNode::Write {
                offset,
                value,
                span,
            } => {
                // TODO: remove clone
                let offset = self.visit_expr(offset, ctx)?;
                let value = self.visit_expr(value, ctx)?;
                if !self.unify(offset.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: offset.span(),
                        expected: vec![FELT_TYPE],
                        found: offset.ty(),
                    });
                }
                if !self.unify(value.ty(), FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: value.span(),
                        expected: vec![FELT_TYPE],
                        found: value.ty(),
                    });
                }
                Ok(CheckedExprNode::Intrinsic(
                    CheckedIntrinsicExprNode::Write {
                        offset: self.program.exprs.alloc_item(offset),
                        value: self.program.exprs.alloc_item(value),
                        type_id: FELT_TYPE,
                        span,
                    },
                ))
            }
            IntrinsicExprNode::Hash { data, span } => {
                let data = self.visit_expr(data, ctx)?;

                Ok(CheckedExprNode::Intrinsic(CheckedIntrinsicExprNode::Hash {
                    data: self.program.exprs.alloc_item(data),
                    type_id: HASH_TYPE,
                    span,
                }))
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_value(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let value_node = ctx.expression(node).as_value().cloned().unwrap();
        match value_node {
            ValueNode::Felt(f, span) => Ok(CheckedExprNode::Value(CheckedValueNode::Felt(
                f.clone(),
                span,
            ))),
            ValueNode::Bool(b, span) => Ok(CheckedExprNode::Value(CheckedValueNode::Bool(
                b.clone(),
                span,
            ))),
            ValueNode::U32(u, span) => Ok(CheckedExprNode::Value(CheckedValueNode::U32(
                u.clone(),
                span,
            ))),
            ValueNode::Array(size, arr, span) => {
                let mut inner_ty = UNKOWN_TYPE;
                let mut elements = Vec::with_capacity(arr.len());
                for e in arr {
                    // TODO: remove clone
                    let checked_expr = self.visit_expr(e, ctx)?;
                    if !self.unify(checked_expr.ty(), inner_ty, ctx) {
                        return Err(Error::TypeMismatch {
                            span: checked_expr.span(),
                            expected: vec![inner_ty],
                            found: checked_expr.ty(),
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

                let &CheckedArrayNode {
                    inner_ty: generic_inner_ty,
                    size_ty: generic_size_ty,
                    ..
                } = ctx.symbols[underlying_type_id].as_array().unwrap();

                if !self.unify(generic_inner_ty, inner_ty, ctx) {
                    return Err(Error::TypeMismatch {
                        span: span,
                        expected: vec![generic_inner_ty],
                        found: inner_ty,
                    });
                }
                if !self.unify(generic_size_ty, size_ty, ctx) {
                    return Err(Error::TypeMismatch {
                        span: span,
                        expected: vec![generic_size_ty],
                        found: size_ty,
                    });
                }

                let type_id = self.substitute_all(underlying_type_id, ctx)?;

                Ok(CheckedExprNode::Value(CheckedValueNode::Array(
                    type_id, elements, span,
                )))
            }
            ValueNode::Struct(name, generic_args, data, span) => Ok({
                let underlying_type_id =
                    ctx.symbols
                        .get_type_id(None, name)
                        .ok_or(Error::UnresolvedType {
                            span: span,
                            resolved_type: name,
                        })?;
                let fields = ctx.symbols[underlying_type_id]
                    .as_struct()
                    .unwrap()
                    .fields
                    .clone();
                let generic_parameters = ctx.symbols[underlying_type_id].generic_parameters();
                if fields.len() != data.len() {
                    return Err(anyhow!(format!(
                        "Expected {} fields for Struct {} but found {} fields",
                        fields.len(),
                        ctx.ident(name),
                        data.len()
                    ))
                    .into());
                }

                let mut new_data = IndexMap::new();
                for (field_name, CheckedStructField { ty: field_type, .. }) in fields {
                    let field_value =
                        self.visit_expr(data.get(&field_name).unwrap().clone(), ctx)?;
                    if !self.unify(field_type, field_value.ty(), ctx) {
                        return Err(Error::TypeMismatch {
                            span: field_value.span(),
                            expected: vec![field_type],
                            found: field_value.ty(),
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
                            span: span,
                            expected: vec![generic_arg],
                            found: generic_param,
                        });
                    }
                }

                let type_id = self.substitute_all(underlying_type_id, ctx)?;

                CheckedExprNode::Value(CheckedValueNode::Struct(type_id, new_data, span))
            }),
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_binary(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let binary_node = ctx.expression(node).as_binary().cloned().unwrap();
        let checked_lhs = self.visit_expr(binary_node.lhs, ctx)?;
        let checked_rhs = self.visit_expr(binary_node.rhs, ctx)?;

        let lhs_ty = checked_lhs.ty();
        if !self.unify(lhs_ty, checked_rhs.ty(), ctx) {
            return Err(Error::TypeMismatch {
                span: checked_rhs.span(),
                expected: vec![lhs_ty],
                found: checked_rhs.ty(),
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
                        span: binary_node.span,
                        expected: vec![FELT_TYPE, U32_TYPE],
                        found: lhs_ty,
                    });
                }
            }
            BinaryOperator::And | BinaryOperator::Or => {
                if !self.unify(lhs_ty, BOOL_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: binary_node.span,
                        expected: vec![BOOL_TYPE],
                        found: lhs_ty,
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
                        span: binary_node.span,
                        expected: vec![BOOL_TYPE, FELT_TYPE, U32_TYPE],
                        found: lhs_ty,
                    });
                }
                BOOL_TYPE
            }
            BinaryOperator::Lt | BinaryOperator::Lte | BinaryOperator::Gt | BinaryOperator::Gte => {
                if !self.unify(lhs_ty, FELT_TYPE, ctx) && !self.unify(lhs_ty, U32_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: binary_node.span,
                        expected: vec![FELT_TYPE, U32_TYPE],
                        found: lhs_ty,
                    });
                }
                BOOL_TYPE
            }
        };

        Ok(CheckedExprNode::Binary(CheckedBinaryNode {
            lhs: self.program.exprs.alloc_item(checked_lhs),
            operator: binary_node.operator,
            rhs: self.program.exprs.alloc_item(checked_rhs),
            type_id: self.substitute_all(type_id, ctx)?,
            span: binary_node.span,
        }))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_unary(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let unary_node = ctx.expression(node).as_unary().cloned().unwrap();
        let checked_expr = self.visit_expr(unary_node.rhs, ctx)?;
        let type_id = checked_expr.ty();

        match unary_node.operator {
            UnaryOperator::Neg => {
                if !self.unify(type_id, FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: unary_node.span,
                        expected: vec![FELT_TYPE],
                        found: type_id,
                    });
                }
            }
            UnaryOperator::Not => {
                if !self.unify(type_id, BOOL_TYPE, ctx) && !self.unify(type_id, FELT_TYPE, ctx) {
                    return Err(Error::TypeMismatch {
                        span: unary_node.span,
                        expected: vec![FELT_TYPE, BOOL_TYPE],
                        found: type_id,
                    });
                }
            }
        }
        if !self.unify(type_id, FELT_TYPE, ctx) && !self.unify(type_id, BOOL_TYPE, ctx) {
            return Err(Error::TypeMismatch {
                span: unary_node.span,
                expected: vec![FELT_TYPE, BOOL_TYPE],
                found: type_id,
            });
        }

        Ok(CheckedExprNode::Unary(CheckedUnaryNode {
            operator: unary_node.operator,
            rhs: self.program.exprs.alloc_item(checked_expr),
            type_id: self.substitute_all(type_id, ctx)?,
            span: unary_node.span,
        }))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_call(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
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
                    span: call_node.span,
                    expected: vec![generic_param.clone()],
                    found: generic_arg,
                });
            }
        }

        let signature = ctx.symbols[ty].signature();

        if call_node.args.len() != signature.parameters.len() {
            return Err(Error::InvalidFunctionCall {
                span: call_node.span,
                method_name: ty,
                expected: format!("{} parameters", signature.parameters.len()),
                found: format!("{}", call_node.args.len()),
            });
        }
        let mut args = Vec::new();
        for (i, arg) in call_node.args.iter().enumerate() {
            let type_arg = self.visit_expr(arg.clone(), ctx)?;
            if !self.unify(type_arg.ty(), signature.parameters[i], ctx) {
                return Err(Error::FunctionParameterMismatch {
                    span: call_node.span,
                    expected: signature.parameters[i],
                    found: type_arg.ty(),
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

        return Ok(checked_expr);
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_member_call(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
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
                    span: call_node.span,
                    expected: vec![generic_param.clone()],
                    found: generic_arg,
                });
            }
        }
        let mut args = Vec::new();
        let receiver = {
            let receiver = self.visit_expr(call_node.receiver, ctx)?;
            if !self.unify(receiver.ty(), f.parameters[0].ty, ctx) {
                return Err(Error::FunctionParameterMismatch {
                    span: call_node.span,
                    expected: f.parameters[0].ty,
                    found: receiver.ty(),
                });
            }

            self.program.exprs.alloc_item(receiver)
        };

        for (i, arg) in call_node.args.iter().enumerate() {
            let type_arg = self.visit_expr(arg.clone(), ctx)?;
            if !self.unify(type_arg.ty(), f.parameters[i + 1].ty, ctx) {
                return Err(Error::FunctionParameterMismatch {
                    span: call_node.span,
                    expected: f.parameters[i + 1].ty,
                    found: type_arg.ty(),
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

        return Ok(checked_expr);
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_tuple(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
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

        let checked_expr = CheckedExprNode::Value(CheckedValueNode::Tuple(
            type_id,
            elements_with_types,
            tuple_node.span,
        ));

        Ok(checked_expr)
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_cast(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
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
                span: cast_node.span,
                expected: vec![target_type],
                found: src_type,
            });
        };
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_if_expr(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        let if_expr_node = ctx.expression(node).as_if_expr().cloned().unwrap();
        let checked_expr = self.visit_expr(if_expr_node.if_branch.predicate, ctx)?;
        if !self.unify(checked_expr.ty(), BOOL_TYPE, ctx) {
            return Err(Error::TypeMismatch {
                span: if_expr_node.span,
                expected: vec![BOOL_TYPE],
                found: checked_expr.ty(),
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
                    span: branch.span,
                    expected: vec![BOOL_TYPE],
                    found: checked_expr.ty(),
                });
            }
            let checked_block = self.visit_expr(branch.body, ctx)?;
            let else_if_type = checked_block.as_block_expr().unwrap().type_id;

            if !self.unify(else_if_type, if_type, ctx) {
                return Err(Error::TypeMismatch {
                    span: branch.span,
                    expected: vec![else_if_type],
                    found: if_type,
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
                    span: if_expr_node.span,
                    expected: vec![if_type],
                    found: else_type,
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
    ) -> StdResult<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let while_node = ctx.statement(node).as_while().cloned().unwrap();
        let predicate = self.visit_expr(while_node.predicate, ctx)?;
        if !self.unify(predicate.ty(), BOOL_TYPE, ctx) {
            return Err(Error::TypeMismatch {
                span: while_node.span,
                expected: vec![BOOL_TYPE],
                found: predicate.ty(),
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
    ) -> StdResult<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let assignment_node = ctx.statement(node).as_assignment().cloned().unwrap();
        let checked_rhs = self.visit_expr(assignment_node.value, ctx)?;
        let checked_lhs = self.visit_expr(assignment_node.target, ctx)?;

        let lhs_ty = checked_lhs.ty();

        if !self.unify(lhs_ty, checked_rhs.ty(), ctx) {
            return Err(Error::TypeMismatch {
                span: assignment_node.span,
                expected: vec![lhs_ty],
                found: checked_rhs.ty(),
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
    ) -> StdResult<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let variable_node = ctx.statement(node).as_variable().cloned().unwrap();
        let lhs_ty = self.typecheck(&variable_node.ty, ctx)?;
        let checked_expr = self.visit_expr(variable_node.value, ctx)?;
        let rhs_ty = checked_expr.ty();
        if !self.unify(rhs_ty, lhs_ty, ctx) {
            return Err(Error::TypeMismatch {
                span: variable_node.span,
                expected: vec![lhs_ty],
                found: rhs_ty,
            });
        }
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        ctx.symbols
            .declare_variable(CheckedVariable::new(
                variable_node.name,
                rhs_ty,
                variable_node.qualifier,
                current_scope_id,
                variable_node.span,
            ))
            .ok_or(error::Error::VariableAlreadyDefined {
                span: variable_node.span,
                variable: variable_node.name,
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
    ) -> StdResult<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let return_node = ctx.statement(node).as_return().cloned().unwrap();
        if !ctx.ancestor_node_type(1).is_block_expr() || !ctx.ancestor_node_type(2).is_function() {
            return Err(Error::InvalidReturn {
                span: return_node.span,
                message: format!("Cannot return from {:?} node", ctx.ancestor_node_type(2)),
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
    ) -> StdResult<Self::StmtResult, Self::Error> {
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
                        span: span,
                        expected: vec![BOOL_TYPE],
                        found: checked_lhs.ty(),
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
                        span: span,
                        expected: vec![checked_lhs.ty()],
                        found: checked_rhs.ty(),
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
    ) -> StdResult<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let impl_node = ctx.definition(node).as_impl().cloned().unwrap();

        let implementor_type_id = ctx.symbols.get_type_id(None, impl_node.ty.name()).unwrap();
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

        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, implementor_type_id)?;

        let mut methods = Vec::new();

        for (generic_parameter, generic_arg) in checked_generic_parameters
            .iter()
            .zip_eq(ctx.symbols[implementor_type_id].generic_parameters())
        {
            if !self.unify(generic_parameter.clone(), generic_arg, ctx) {
                return Err(Error::TypeMismatch {
                    span: impl_node.span,
                    expected: vec![generic_parameter.clone()],
                    found: generic_arg,
                });
            }
        }

        for &function_id in &impl_node.body {
            methods.push(CheckedDefinitionNode::Function(
                self.typecheck_impl_method(implementor_type_id, function_id, ctx)?,
            ));
        }

        let checked_impl = CheckedImplNode {
            generic_parameters: checked_generic_parameters,
            ty: implementor_type_id,
            body: self.program.defs.alloc_items(methods),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
        };
        // let ty = Type::Impl(checked_impl.clone());
        // symbols.add_type(symbols.parent_scope_id(), ty);

        self.infcx.exit_context();
        ctx.symbols.end_scope();

        let impl_id = self
            .program
            .defs
            .alloc_item(CheckedDefinitionNode::Impl(checked_impl));
        self.register_impl(impl_id, ctx)?;

        Ok(impl_id)
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_trait(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::DefinitionResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Trait);
        // ctx.symbols.start_scope(ScopeKind::Impl);
        self.infcx.enter_context();
        // TODO: remove clone
        let trait_node = ctx.definition(node).as_trait().cloned().unwrap();

        let mut generic_parameters = Vec::new();
        let mut methods = Vec::new();

        for generic_parameter in &trait_node.generic_parameters {
            let checked_generic_parameter =
                self.typecheck_generic_parameter(generic_parameter, ctx)?;
            let type_id = ctx
                .symbols
                .add_type_variable(ScopeKind::Trait, checked_generic_parameter)?;
            generic_parameters.push(type_id);
        }

        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, UNKOWN_TYPE)?;

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
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            visibility: trait_node.visibility,
            span: trait_node.span,
        };
        // TODO: remove clone
        let ty = Type::Trait(checked_trait.clone());
        ctx.symbols
            .add_type(ctx.symbols.parent_scope_id(), checked_trait.name, ty)?;

        self.infcx.exit_context();
        // ctx.symbols.end_scope();
        ctx.symbols.end_scope();

        Ok(self
            .program
            .defs
            .alloc_item(CheckedDefinitionNode::Trait(checked_trait)))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_function(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::DefinitionResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Function);
        self.infcx.enter_context();
        let function = ctx.definition(node).as_function().cloned().unwrap();

        let mut checked_generic_parameters = Vec::with_capacity(function.generic_parameters.len());
        for generic_parameter in &function.generic_parameters {
            let checked_generic_parameter =
                self.typecheck_generic_parameter(generic_parameter, ctx)?;
            let type_id = ctx
                .symbols
                .add_type_variable(ScopeKind::Function, checked_generic_parameter)?;
            checked_generic_parameters.push(type_id);
        }

        let checked_function =
            self.typecheck_function(function, checked_generic_parameters, ctx)?;
        if checked_function.body.is_none() {
            return Err(Error::FunctionHasNoBody {
                span: checked_function.span,
                function: checked_function.name,
            });
        }
        let ty = Type::Function(checked_function.clone());
        ctx.symbols
            .add_type(ctx.symbols.parent_scope_id(), ty.name(), ty)?;
        self.infcx.exit_context();
        ctx.symbols.end_scope();

        Ok(self
            .program
            .defs
            .alloc_item(CheckedDefinitionNode::Function(checked_function)))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_struct(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::DefinitionResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Struct);
        self.infcx.enter_context();
        // TODO: remove clone
        let struct_node = ctx.definition(node).as_struct().cloned().unwrap();

        let mut generic_parameters = Vec::new();

        for generic_parameter in struct_node.generic_parameters {
            let checked_generic_parameter =
                self.typecheck_generic_parameter(&generic_parameter, ctx)?;
            let type_id = ctx
                .symbols
                .add_type_variable(ScopeKind::Struct, checked_generic_parameter)?;
            generic_parameters.push(type_id);
        }

        let mut checked_struct = CheckedStructNode {
            name: struct_node.name.clone(),
            generic_parameters,
            fields: IndexMap::new(),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            visibility: struct_node.visibility,
            span: struct_node.span,
        };

        for (
            field_name,
            StructField {
                ty: field_type,
                visibility,
                span,
            },
        ) in &struct_node.fields
        {
            let ty = self.typecheck(&field_type, ctx)?;
            checked_struct.fields.insert(
                field_name.clone(),
                CheckedStructField {
                    ty,
                    visibility: visibility.clone(),
                    span: span.clone(),
                },
            );
        }

        let ty = Type::Struct(checked_struct.clone());
        ctx.symbols
            .add_type(ctx.symbols.parent_scope_id(), checked_struct.name, ty)?;

        self.infcx.exit_context();
        ctx.symbols.end_scope();

        Ok(self
            .program
            .defs
            .alloc_item(CheckedDefinitionNode::Struct(checked_struct)))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_enum(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::DefinitionResult, Self::Error> {
        ctx.symbols.start_scope(ScopeKind::Enum);
        self.infcx.enter_context();
        // TODO: remove clone
        let enum_node = ctx.definition(node).as_enum().cloned().unwrap();
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();

        let mut generic_parameters = Vec::new();

        for generic_parameter in enum_node.generic_parameters {
            let checked_generic_parameter =
                self.typecheck_generic_parameter(&generic_parameter, ctx)?;
            let type_id = ctx
                .symbols
                .add_type_variable(ScopeKind::Enum, checked_generic_parameter)?;
            generic_parameters.push(type_id);
        }

        let checked_enum = CheckedEnumNode {
            generic_parameters,
            name: enum_node.name,
            variants: Vec::new(),
            scope_id: current_scope_id,
            visibility: enum_node.visibility,
            span: enum_node.span,
        };
        let ty = Type::Enum(checked_enum.clone());
        ctx.symbols
            .add_type(ctx.symbols.parent_scope_id(), checked_enum.name, ty)?;

        self.infcx.exit_context();
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
    ) -> StdResult<Self::ExprResult, Self::Error> {
        ctx.push_node_id(NodeId::from(expr_id));
        self.infcx.enter_scope();
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
        self.infcx.exit_scope();
        ctx.pop_node_id();
        Ok(res)
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_definition(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::DefinitionResult, Self::Error> {
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
    ) -> StdResult<Self::StmtResult, Self::Error> {
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
                definition
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
    ) -> StdResult<(), Self::Error> {
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
    fn visit_program(&mut self, ctx: &mut Self::Context) -> StdResult<(), Self::Error> {
        // TODO: remove clone
        ctx.symbols
            .load_modules(ctx.program().modules.clone().iter());
        let mut colors = HashMap::new();
        ctx.dependency_graph().ts::<Self::Error>(
            &ModuleId::root(),
            &mut colors,
            &mut |&module_id| {
                ctx.symbols.enter_module(module_id);
                self.visit_module(module_id, ctx)?;
                ctx.symbols.exit_module();

                Ok(())
            },
        )?;

        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_block_expr(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
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
                    span: checked_stmt.as_return().unwrap().span,
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
    ) -> StdResult<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let node = ctx.definition(node).as_type_alias().cloned().unwrap();

        let type_id = self.typecheck(&node.ty, ctx)?;
        let mut key: TypeKey = node.name.into();
        key.visibility = node.visibility;
        ctx.symbols.add_type_id(None, key, type_id)?;

        Ok(self
            .program
            .defs
            .alloc_item(CheckedDefinitionNode::TypeAlias(CheckedTypeAliasNode {
                name: node.name,
                ty: type_id,
                visibility: node.visibility,
            })))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_const(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let node = ctx.definition(node).as_const().cloned().unwrap();
        let lhs_ty = self.typecheck(&node.ty, ctx)?;
        let value = self.visit_expr(node.value, ctx)?;
        let rhs_ty = value.ty();
        if !self.unify(lhs_ty, rhs_ty, ctx) {
            return Err(Error::TypeMismatch {
                span: node.span,
                expected: vec![rhs_ty],
                found: lhs_ty,
            });
        }

        let value = self.evaluator.evaluate_expr(&self.program, &value, ctx);

        let node = CheckedConstNode {
            name: Some(node.name),
            ty: rhs_ty,
            value: ctx.symbols.add_constant(value),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            visibility: node.visibility,
        };

        let ty = Type::Const(node.clone());

        ctx.symbols.add_type(None, ty.key(), ty)?;

        Ok(self
            .program
            .defs
            .alloc_item(CheckedDefinitionNode::Const(node)))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_for(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::StmtResult, Self::Error> {
        // TODO: remove clone
        let for_node = ctx.statement(node).as_for().cloned().unwrap();
        ctx.symbols.start_scope(ScopeKind::Block);
        let start = self.visit_expr(for_node.start, ctx)?;
        let end = self.visit_expr(for_node.end, ctx)?;
        if !(self.unify(start.ty(), FELT_TYPE, ctx) && self.unify(end.ty(), FELT_TYPE, ctx)
            || self.unify(start.ty(), U32_TYPE, ctx) && self.unify(end.ty(), U32_TYPE, ctx))
        {
            return Err(Error::TypeMismatch {
                span: for_node.span,
                expected: vec![FELT_TYPE, U32_TYPE],
                found: start.ty(),
            });
        }

        let current_scope_id = ctx.symbols.current_scope_id().unwrap();
        let variable = CheckedVariable::new(
            for_node.variable,
            start.ty(),
            TypeQualifier::new(true),
            current_scope_id,
            for_node.span,
        );
        ctx.symbols
            .declare_variable(variable)
            .ok_or(error::Error::VariableAlreadyDefined {
                span: for_node.span,
                variable: for_node.variable,
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
    ) -> StdResult<Self::ExprResult, Self::Error> {
        //get match node
        let match_node = ctx.expression(node).as_match().cloned().unwrap();
        let checked_scrutinee = self.visit_expr(match_node.scrutinee, ctx)?;

        //There are two type constraints here, one is that the scrutinee_type must be consistent with the type of the value of the pattern
        //The other is that the return types of all cases must be consistent
        let scrutinee_type = checked_scrutinee.ty();
        let white_list = vec![FELT_TYPE, BOOL_TYPE, U32_TYPE];
        if !white_list.contains(&scrutinee_type) {
            return Err(Error::TypeMismatch {
                span: match_node.span,
                expected: white_list,
                found: scrutinee_type,
            });
        }

        if scrutinee_type == BOOL_TYPE {
            if !match_node.arms.len() == 2 {
                return Err(Error::IncompleteMatch {
                    span: match_node.span,
                    message: "Boolean match must have 2 arms".to_string(),
                });
            }
        }
        let mut checked_arms = Vec::new();
        let mut match_expr_type: Option<TypeId> = None;
        let mut wildcard_case: Option<CheckedMatchArm> = None;

        for (_idx, arm) in match_node.arms.iter().enumerate() {
            let checked_pattern = match &arm.pattern {
                MatchPattern::Value(pattern_expr, _pattern_span) => {
                    let checked_pattern_expr = self.visit_expr(*pattern_expr, ctx)?;
                    let pattern_type = checked_pattern_expr.ty();

                    if !self.unify(scrutinee_type, pattern_type, ctx) {
                        //todo!: When the span of value is implemented, you need to refactor here
                        let span = checked_pattern_expr.span();
                        return Err(Error::TypeMismatch {
                            span: span,
                            expected: vec![scrutinee_type],
                            found: pattern_type,
                        });
                    }
                    Some(self.program.exprs.alloc_item(checked_pattern_expr))
                }
                MatchPattern::PlaceHolder(span) => {
                    if wildcard_case.is_some() {
                        return Err(Error::DuplicateWildcard { span: span.clone() });
                    }
                    None
                }
            };

            let checked_body = self.visit_expr(arm.body, ctx)?;
            let arm_body_type = checked_body.ty();
            match_expr_type.get_or_insert(arm_body_type);
            if !self.unify(match_expr_type.unwrap(), arm_body_type, ctx) {
                let span = checked_body.span();
                return Err(Error::TypeMismatch {
                    span: span,
                    expected: vec![match_expr_type.unwrap()],
                    found: arm_body_type,
                });
            }

            let checked_arm = CheckedMatchArm {
                pattern: checked_pattern,
                body: self.program.exprs.alloc_item(checked_body),
                span: arm.span,
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
            span: match_node.span,
        }))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_lambda_function(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::ExprResult, Self::Error> {
        // TODO: remove clone
        ctx.symbols.start_scope(ScopeKind::LambdaFunction);
        let function = ctx.expression(node).as_lambda_function().cloned().unwrap();

        let current_scope_id = ctx.symbols.current_scope_id().unwrap();

        let mut parameters = Vec::new();

        for parameter in &function.parameters {
            let parameter_type = self.typecheck(&parameter.ty, ctx)?;
            let variable = CheckedVariable::new(
                parameter.name,
                parameter_type,
                parameter.qualifier,
                current_scope_id,
                parameter.span,
            );
            ctx.symbols
                .declare_variable(variable)
                .ok_or(error::Error::VariableAlreadyDefined {
                    span: function.span,
                    variable: parameter.name.clone(),
                })?;
            parameters.push(CheckedFunctionParameter::new(
                parameter.name,
                parameter.qualifier,
                parameter_type,
                parameter.span,
            ));
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
                    span: function.span,
                    expected: vec![expected_return_type],
                    found: actual_return_type,
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
        ctx.symbols[type_id]
            .as_lambda_function_mut()
            .unwrap()
            .type_id = type_id;
        checked_function.type_id = type_id;

        ctx.symbols.end_scope();
        Ok(CheckedExprNode::LambdaFunction(checked_function))
    }

    #[instrument(level = "debug", skip_all)]
    fn visit_impl_trait(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> StdResult<Self::DefinitionResult, Self::Error> {
        // TODO: remove clone
        let impl_node = ctx.definition(node).as_impl_trait().cloned().unwrap();

        let trait_type_id = self.typecheck(&impl_node.trait_ty, ctx)?;
        let implementor_type_id = ctx.symbols.get_type_id(None, impl_node.ty.name()).unwrap();

        ctx.symbols.start_scope(ScopeKind::Impl);
        self.infcx.enter_context();

        ctx.symbols
            .add_type_id(None, IdentId::TYPE_SELF, implementor_type_id)?;

        let trait_node = ctx.symbols[trait_type_id].clone().into_trait().unwrap();
        let mut generic_parameters = Vec::new();
        let mut unimplemented_methods: HashSet<DefId> =
            trait_node.unchecked_body.iter().cloned().collect();
        let mut checked_methods = Vec::with_capacity(trait_node.body.len());

        for generic_parameter in &impl_node.generic_parameters {
            let checked_generic_parameter =
                self.typecheck_generic_parameter(generic_parameter, ctx)?;
            let type_id = ctx
                .symbols
                .add_type_variable(ScopeKind::Impl, checked_generic_parameter)?;
            generic_parameters.push(type_id);
        }

        for &function_id in &impl_node.body {
            let method = self.typecheck_impl_trait_method(
                trait_type_id,
                implementor_type_id,
                function_id,
                ctx,
            )?;
            let i = trait_node
                .body
                .iter()
                .position(|&trait_def_id| {
                    let trait_function = self.program.defs[trait_def_id].as_function().unwrap();
                    trait_function.trait_impl_signature(implementor_type_id) == method.signature()
                        && trait_function.name == method.name
                })
                .ok_or(Error::UnresolvedTraitMethod {
                    method_span: method.span,
                    trait_name: trait_node.name,
                    method_name: method.name,
                })?;
            unimplemented_methods.remove(&trait_node.unchecked_body[i]);
            checked_methods.push(CheckedDefinitionNode::Function(method));
        }

        for unimplemented_method in unimplemented_methods {
            let method = self.typecheck_impl_trait_method(
                trait_type_id,
                implementor_type_id,
                unimplemented_method,
                ctx,
            )?;
            if method.body.is_none() {
                return Err(Error::TraitMethodUnimplemented {
                    span: method.span,
                    trait_ty: trait_type_id,
                    ty: implementor_type_id,
                    method: method.name,
                });
            }
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

        self.infcx.exit_context();
        ctx.symbols.end_scope();

        let trait_impl_id = self
            .program
            .defs
            .alloc_item(CheckedDefinitionNode::ImplTrait(checked_impl));
        self.register_trait_impl(trait_impl_id, ctx)?;

        Ok(trait_impl_id)
    }
}

impl<F: Clone + From<u32> + ContextFelt, C> TypeChecker<F, C> {
    pub fn new(program: CheckedProgram<F>, evaluator: Box<dyn Evaluator<F, C>>) -> Self {
        Self {
            program,
            evaluator,
            resolver: ResolverCtxt::new(),
            infcx: InferCtxt::new(),
            implementer: ImplementerCtxt::new(),
            _marker: std::marker::PhantomData,
        }
    }

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
            value: ctx.symbols.add_constant(CheckedValueRef::from_u32(value)),
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            visibility: Visibility::Public,
        };

        ctx.symbols
            .get_or_add_type(None, TypeKey::from(node.value), Type::Const(node))
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_array(&mut self, ctx: &mut TypeCheckerVisitorContext<F, C>) -> Result<TypeId> {
        ctx.symbols.start_scope(ScopeKind::Array);
        self.infcx.enter_context();

        let inner_ty = ctx.symbols.add_type_variable(
            ScopeKind::Module,
            CheckedGenericParameter::new(
                IdentId::T,
                vec![],
                ScopeId::primitive(),
                Span {
                    file_id: FileId(0),
                    start: 0,
                    end: 0,
                },
            ),
        )?;
        let size = ctx.symbols.add_type_variable(
            ScopeKind::Module,
            CheckedGenericParameter::new(
                IdentId::N,
                vec![],
                ScopeId::primitive(),
                Span {
                    file_id: FileId(0),
                    start: 0,
                    end: 0,
                },
            ),
        )?;

        let checked_array = CheckedArrayNode {
            inner_ty,
            size_ty: size,
            scope_id: ctx.symbols.current_scope_id().unwrap(),
        };

        let ty = Type::Array(checked_array.clone());
        let type_id = ctx
            .symbols
            .add_type(ctx.symbols.parent_scope_id(), ty.name(), ty)?;

        self.infcx.exit_context();
        ctx.symbols.end_scope();
        Ok(type_id)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck(
        &mut self,
        ty: &UncheckedType,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        self.infcx.enter_scope();
        let type_id = match ty {
            UncheckedType::Basic(IdentId::TYPE_BOOL, _) => BOOL_TYPE,
            UncheckedType::Basic(IdentId::TYPE_FELT, _) => FELT_TYPE,
            UncheckedType::Basic(IdentId::TYPE_U32, _) => U32_TYPE,
            UncheckedType::Basic(name, span) => {
                ctx.symbols
                    .get_type_id(None, name.clone())
                    .ok_or(Error::UnresolvedType {
                        span: span.clone(),
                        resolved_type: name.clone(),
                    })?
            }
            UncheckedType::Generic(name, generic_parameters, span) => {
                let underlying_type_id =
                    ctx.symbols
                        .get_type_id(None, name.clone())
                        .ok_or(Error::UnresolvedType {
                            span: span.clone(),
                            resolved_type: name.clone(),
                        })?;

                let mut checked_generic_args = Vec::new();
                for generic_parameter in generic_parameters {
                    checked_generic_args.push(self.typecheck(generic_parameter, ctx)?);
                }

                match ctx.symbols[underlying_type_id].clone() {
                    Type::Struct(checked_struct) => {
                        if checked_struct.generic_parameters.len() != checked_generic_args.len() {
                            return Err(Error::GenericParameterMismatch {
                                span: checked_struct.span,
                                expected: format!(
                                    "{} generic parameters",
                                    checked_struct.generic_parameters.len()
                                ),
                                found: format!("{}", checked_generic_args.len()),
                            });
                        }

                        for (generic_param, generic_arg) in checked_struct
                            .generic_parameters
                            .iter()
                            .zip(checked_generic_args.iter())
                        {
                            if !self.unify(*generic_param, *generic_arg, ctx) {
                                return Err(Error::TypeMismatch {
                                    span: span.clone(),
                                    expected: vec![*generic_param],
                                    found: *generic_arg,
                                });
                            }
                        }

                        self.substitute_all(underlying_type_id, ctx)?
                    }

                    Type::Array(checked_array) => {
                        if checked_generic_args.len() != 2 {
                            return Err(Error::GenericParameterMismatch {
                                span: span.clone(),
                                expected: format!("2 generic parameters",),
                                found: format!("{}", checked_generic_args.len()),
                            });
                        }
                        if !self.unify(checked_array.inner_ty, checked_generic_args[0], ctx) {
                            return Err(Error::TypeMismatch {
                                span: span.clone(),
                                expected: vec![checked_array.inner_ty],
                                found: checked_generic_args[0],
                            });
                        }
                        if !self.unify(checked_array.size_ty, checked_generic_args[1], ctx) {
                            return Err(Error::TypeMismatch {
                                span: span.clone(),
                                expected: vec![checked_array.size_ty],
                                found: checked_generic_args[1],
                            });
                        }
                        self.substitute_all(underlying_type_id, ctx)?
                    }
                    _ => unreachable!(),
                }
            }
            UncheckedType::Array(inner_ty, size, span) => {
                let underlying_type_id = ctx
                    .symbols
                    .get_type_id(Some(ScopeId::primitive()), IdentId::TYPE_ARRAY)
                    .unwrap();

                let &CheckedArrayNode {
                    inner_ty: generic_inner_ty,
                    size_ty: generic_size_ty,
                    ..
                } = ctx.symbols[underlying_type_id].as_array().unwrap();

                let inner_ty = self.typecheck(inner_ty.as_ref(), ctx)?;

                let size_ty = self.populate_constant_u32(size.clone(), ctx)?;

                if !self.unify(generic_inner_ty, inner_ty, ctx) {
                    return Err(Error::TypeMismatch {
                        span: span.clone(),
                        expected: vec![generic_inner_ty],
                        found: inner_ty,
                    });
                }
                if !self.unify(generic_size_ty, size_ty, ctx) {
                    return Err(Error::TypeMismatch {
                        span: span.clone(),
                        expected: vec![generic_size_ty],
                        found: size_ty,
                    });
                }

                self.substitute_all(underlying_type_id, ctx)?
            }
            UncheckedType::Tuple(elements, _) => {
                // check each element and collect results into a Result<Vec<TypeId>>
                let checked_elements = elements
                    .iter()
                    .map(|elem_ty| self.typecheck(elem_ty, ctx))
                    .collect::<Result<_>>()?;

                let checked_tuple = Type::Tuple(checked_elements);

                let scope_id = ScopeId::primitive();

                ctx.symbols
                    .get_or_add_type(Some(scope_id), checked_tuple.key(), checked_tuple)?
            }
            UncheckedType::Unknown => UNKOWN_TYPE,
            UncheckedType::FunctionSignature(function_signature, _) => {
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

                ctx.symbols.get_or_add_type(None, ty.key(), ty)?
            }
        };
        self.infcx.exit_scope();
        Ok(type_id)
    }

    fn typecheck_generic_parameter(
        &mut self,
        ty: &GenericParameter,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedGenericParameter> {
        let mut constraints = Vec::with_capacity(ty.constraints.len());
        for constraint in &ty.constraints {
            let constraint = self.typecheck(constraint, ctx)?;
            constraints.push(constraint);
        }

        Ok(CheckedGenericParameter {
            name: ty.name,
            constraints,
            scope_id: ctx.symbols.current_scope_id().unwrap(),
            span: ty.span,
        })
    }

    pub fn add_use(
        &mut self,
        use_path: &UseNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> StdResult<(), Error> {
        let type_ids = self.resolve_use(&use_path, ctx)?;
        let type_ids = type_ids
            .into_iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();
        for (mut key, type_id) in type_ids {
            key.visibility = use_path.visibility;
            let _ = ctx.symbols.add_type_id(None, key.clone(), type_id);
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_trait_method(
        &mut self,
        function_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        ctx.push_node_id(NodeId::from(function_id));
        ctx.symbols.start_scope(ScopeKind::TraitMethod);
        self.infcx.enter_scope();

        let function = ctx.definition(function_id).as_function().cloned().unwrap();
        let mut checked_generic_parameters = Vec::with_capacity(function.generic_parameters.len());
        for generic_parameter in &function.generic_parameters {
            let checked_generic_parameter =
                self.typecheck_generic_parameter(generic_parameter, ctx)?;
            let type_id = ctx
                .symbols
                .add_type_variable(ScopeKind::Trait, checked_generic_parameter)?;
            checked_generic_parameters.push(type_id);
        }
        let checked_function =
            self.typecheck_function(function, checked_generic_parameters, ctx)?;
        let ty = Type::Function(checked_function.clone());
        ctx.symbols.add_type(None, checked_function.name, ty)?;

        self.infcx.exit_scope();
        ctx.symbols.end_scope();
        ctx.pop_node_id();
        Ok(checked_function)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_impl_trait_method(
        &mut self,
        trait_type_id: TypeId,
        implementor_type_id: TypeId,
        function_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        ctx.push_node_id(NodeId::from(function_id));
        ctx.symbols.start_scope(ScopeKind::ImplMethod);
        self.infcx.enter_scope();

        let function = ctx.definition(function_id).as_function().cloned().unwrap();
        let mut checked_generic_parameters = Vec::with_capacity(function.generic_parameters.len());
        for generic_parameter in &function.generic_parameters {
            let checked_generic_parameter =
                self.typecheck_generic_parameter(generic_parameter, ctx)?;
            let type_id = ctx
                .symbols
                .add_type_variable(ScopeKind::Impl, checked_generic_parameter)?;
            checked_generic_parameters.push(type_id);
        }
        let checked_function =
            self.typecheck_function(function, checked_generic_parameters, ctx)?;
        let ty = Type::Function(checked_function.clone());
        let type_id = ctx.symbols.add_type(None, checked_function.name, ty)?;

        self.infcx.exit_scope();
        ctx.symbols.end_scope();
        ctx.pop_node_id();
        Ok(checked_function)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_impl_method(
        &mut self,
        implementor_type_id: TypeId,
        function_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        ctx.push_node_id(NodeId::from(function_id));
        ctx.symbols.start_scope(ScopeKind::ImplMethod);
        self.infcx.enter_scope();

        let function = ctx.definition(function_id).as_function().cloned().unwrap();
        let mut checked_generic_parameters = Vec::with_capacity(function.generic_parameters.len());
        for generic_parameter in &function.generic_parameters {
            let checked_generic_parameter =
                self.typecheck_generic_parameter(generic_parameter, ctx)?;
            let type_id = ctx
                .symbols
                .add_type_variable(ScopeKind::Impl, checked_generic_parameter)?;
            checked_generic_parameters.push(type_id);
        }
        let checked_function =
            self.typecheck_function(function, checked_generic_parameters, ctx)?;
        if checked_function.body.is_none() {
            return Err(Error::MethodHasNoBody {
                span: checked_function.span,
                ty: implementor_type_id,
                method: checked_function.name,
            });
        }
        let ty = Type::Function(checked_function.clone());
        let type_id = ctx.symbols.add_type(None, checked_function.name, ty)?;

        self.infcx.exit_scope();
        ctx.symbols.end_scope();
        ctx.pop_node_id();
        Ok(checked_function)
    }

    #[instrument(level = "debug", skip_all)]
    fn typecheck_function(
        &mut self,
        function: FunctionNode,
        generic_parameters: Vec<TypeId>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedFunctionNode> {
        let current_scope_id = ctx.symbols.current_scope_id().unwrap();

        let mut parameters = Vec::new();

        for parameter in &function.parameters {
            let parameter_type = self.typecheck(&parameter.ty, ctx)?;
            let variable = CheckedVariable::new(
                parameter.name,
                parameter_type,
                parameter.qualifier,
                current_scope_id,
                parameter.span,
            );
            ctx.symbols
                .declare_variable(variable)
                .ok_or(error::Error::VariableAlreadyDefined {
                    span: function.span,
                    variable: parameter.name,
                })?;
            parameters.push(CheckedFunctionParameter::new(
                parameter.name,
                parameter.qualifier,
                parameter_type,
                parameter.span,
            ));
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
                    span: function.span,
                    expected: vec![expected_return_type],
                    found: actual_return_type,
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
}
