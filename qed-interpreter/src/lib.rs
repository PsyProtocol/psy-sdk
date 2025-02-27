#![feature(try_trait_v2)]

mod control;
mod error;
mod preprocess;

use crate::control::ControlState;
use error::{Error, Result};
use indexmap::IndexMap;
use plonky2::field::goldilocks_field::GoldilocksField;
pub use preprocess::StorageProcessor;
use qed_ast::*;
use qed_crypto::hash::utils::gen_dapen_contract_function_method_id;
use qed_fmt::Formatter;
use qed_parser::Parser;
use qed_sema::CheckedBlockExprNode;
use qed_sema::CheckedIfExprNode;
use qed_sema::Error as SemaError;
use qed_sema::*;
use qedlang_core::dpn::{
    ops::{
        context_trait::{ContextFelt, DPNContext, ToFelts},
        op_types::DPNOpType,
    },
    vm::def::DPNFunctionCircuitDefinition,
};
use std::iter::once;

use std::{collections::HashMap, path::PathBuf};
// use tracing::{debug, error, info, instrument, span, Level};
use tracing::instrument;

#[allow(dead_code)]
type GF = GoldilocksField;
#[derive(Debug)]
pub struct Interpreter<F: Clone + From<u32>, C> {
    pub context: C,
    _marker: std::marker::PhantomData<F>,
}

impl<F: ContextFelt + From<u32>, C: DPNContext<F>> Interpreter<F, C> {
    pub fn new(context: C) -> Self {
        Self {
            context,
            _marker: std::marker::PhantomData,
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret<I: Into<Ident>>(
        &mut self,
        entry: PathBuf,
        contract_name: Option<I>,
        method_names: Vec<I>,
        compile_fn: impl Fn(&C, (String, u32, Vec<F>)) -> DPNFunctionCircuitDefinition,
    ) -> Result<Vec<DPNFunctionCircuitDefinition>>
    where
        F: 'static,
    {
        let (typechecker, mut typechecker_context) = self.typecheck(entry)?;

        let TypeCheckerVisitorContext {
            ref mut program,
            ref mut symbols,
            ..
        } = typechecker_context;

        let scope_id = symbols[ModuleId::root()].scope_id;
        let type_ids = if let Some(contract_name) = contract_name {
            let contract_name = program.interner.intern_ident(contract_name.into());
            let type_id = symbols[scope_id]
                .types
                .get(&contract_name.into())
                .ok_or(Error::UndefinedFunction)?
                .clone();

            method_names
                .into_iter()
                .map(|method_name| {
                    let method_name = program.interner.intern_ident(method_name.into());
                    symbols
                        .resolve_method(type_id, method_name)
                        .ok_or(Error::from(SemaError::UnresolvedMember))
                })
                .collect::<Result<Vec<TypeId>>>()?
        } else {
            method_names
                .into_iter()
                .map(|method_name| {
                    let method_name = program.interner.intern_ident(method_name.into());
                    symbols[scope_id]
                        .types
                        .get(&method_name.into())
                        .ok_or(Error::UndefinedFunction)
                        .cloned()
                })
                .collect::<Result<Vec<TypeId>>>()?
        };

        let mut outputs = Vec::new();
        // backup context
        let context = self.context.clone();

        for type_id in type_ids {
            let node: &CheckedFunctionNode = symbols[type_id.clone()].as_ref();

            let mut parameters = vec![];
            for (_, _, parameter_type) in node.parameters.iter() {
                let ty = &symbols[parameter_type.clone()];
                parameters.push(CheckedValueRef::new_rc(
                    ty.to_value(&symbols, &mut self.context),
                ));
            }
            let res = self.__interpret__(&typechecker, program, symbols, type_id, parameters)?;
            outputs.push(compile_fn(&self.context, res));

            // restore context
            self.context = context.clone();
        }

        Ok(outputs)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn test(
        &mut self,
        entry: PathBuf,
        compile_fn: impl Fn(&C, (String, u32, Vec<F>)) -> DPNFunctionCircuitDefinition,
    ) -> Result<Vec<DPNFunctionCircuitDefinition>>
    where
        F: 'static,
    {
        let (typechecker, mut typechecker_context) = self.typecheck(entry)?;

        let TypeCheckerVisitorContext {
            ref mut program,
            ref mut symbols,
            ..
        } = typechecker_context;

        let mut type_ids = Vec::new();

        let mut visited = HashMap::new();
        program
            .dependency_graph
            .clone()
            .ts(&ModuleId::root(), &mut visited, &mut |&module_id| {
                let scope_id = symbols[module_id].scope_id;
                let functions = symbols[scope_id]
                    .types
                    .iter()
                    .filter(|(_, &v)| {
                        symbols[v.clone()].is_function()
                            && symbols[v.clone()]
                                .as_function()
                                .map(|x| x.attrs.iter().any(|y| y.is_test()))
                                .unwrap_or(false)
                    })
                    .map(|(_, v)| v.clone())
                    .collect::<Vec<_>>();
                type_ids.push((module_id, functions));
            })
            .unwrap();

        let mut outputs = Vec::new();
        // backup context
        let context = self.context.clone();
        for (_, type_ids) in type_ids {
            for type_id in type_ids {
                assert!(symbols[type_id]
                    .as_function()
                    .unwrap()
                    .parameters
                    .is_empty());
                let res = self.__interpret__(&typechecker, program, symbols, type_id, vec![])?;
                outputs.push(compile_fn(&self.context, res));
                // resotre context
                self.context = context.clone();
            }
        }

        Ok(outputs)
    }

    fn __interpret__(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        program: &mut Program<F>,
        symbols: &mut SymbolTable<F>,
        type_id: TypeId,
        parameters: Vec<CheckedValueRef<F>>,
    ) -> Result<(String, u32, Vec<F>)>
    where
        F: 'static,
    {
        let node: &CheckedFunctionNode = symbols[type_id.clone()].as_ref();
        let method_name = program[node.name].to_string();
        let mut method_args = Vec::with_capacity(node.parameters.len());

        for (parameter_name, _, parameter_type) in node.parameters.iter() {
            method_args.push((
                program[parameter_name.clone()].to_string(),
                symbols.size_of(parameter_type.clone()),
            ));
        }

        let outputs = self
            .interpret_function(&typechecker, type_id, parameters, symbols)?
            .unwrap();

        let method_id = gen_dapen_contract_function_method_id(method_name.clone(), &method_args);

        Ok((
            method_name,
            method_id,
            outputs.map(|x| x.to_felts()).unwrap_or(Vec::new()),
        ))
    }

    pub fn typecheck(
        &mut self,
        entry: PathBuf,
    ) -> Result<(TypeChecker<F, C>, TypeCheckerVisitorContext<F, C>)>
    where
        F: 'static,
    {
        let mut typechecker = TypeChecker::new();
        let mut program = Program::new();
        let mut parser = Parser::new(&mut program);
        parser
            .parse(&mut self.context, entry)
            .map_err(|err| Error::ParseError(err.to_string()))?;

        let mut storage_preprocessor: StorageProcessor = StorageProcessor::new();
        let mut default_visitor_context: DefaultVisitorContext<'_, F, C> =
            DefaultVisitorContext::new(&mut program);
        storage_preprocessor
            .visit_program(&mut default_visitor_context)
            .unwrap();

        let mut formatter = Formatter::new();
        formatter
            .visit_program(&mut default_visitor_context)
            .unwrap();
        println!("formatted:\n{}", formatter.get_output());

        let mut typechecker_context = TypeCheckerVisitorContext::new(program);
        typechecker.visit_program(&mut typechecker_context)?;
        Ok((typechecker, typechecker_context))
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_function(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        type_id: TypeId,
        parameters: Vec<CheckedValueRef<F>>,
        symbols: &mut SymbolTable<F>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        symbols.push_frame();
        let res = self.__interpret_function__(typechecker, type_id, parameters, symbols);
        symbols.pop_frame();
        res
    }

    fn __interpret_function__(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        type_id: TypeId,
        parameters: Vec<CheckedValueRef<F>>,
        symbols: &mut SymbolTable<F>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        // TODO: remove clone
        let node = symbols[type_id].as_function().cloned().unwrap();

        assert_eq!(
            parameters.len(),
            node.parameters.len(),
            "expceted {} parameters for main function, got {}",
            node.parameters.len(),
            parameters.len()
        );

        for (i, (parameter, _, _)) in node.parameters.iter().enumerate() {
            symbols.set_variable(node.scope_id, parameter, parameters[i].clone())?;
        }
        self.interpret_statement(typechecker, node.body.unwrap(), symbols)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_while(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
    ) -> Result<()> {
        let node = typechecker[stmt_id].as_while().unwrap();
        loop {
            let predicate = self
                .interpret_expr(typechecker, node.predicate, symbols)?
                .unwrap()
                .to_bool();
            let constant_types = [
                DPNOpType::Constant,
                DPNOpType::ConstantTrue,
                DPNOpType::ConstantTrue,
            ];

            if !constant_types.contains(&self.context.get_op_type(predicate)) {
                return Err(Error::UncertainLoopCondition);
            }

            if self.context.get_constant_value(predicate) != 0 {
                self.context.start_if_block(predicate);
                self.interpret_statement(typechecker, node.body, symbols)?;
                self.context.end_if_block();
            } else {
                break Ok(());
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_statement(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        let node = &typechecker[stmt_id];
        match node {
            CheckedStmtNode::While(r#_while) => {
                self.interpret_while(typechecker, stmt_id, symbols)?
            }
            CheckedStmtNode::Assignment(r#_assignment) => {
                self.interpret_assignment(typechecker, stmt_id, symbols)?
            }
            CheckedStmtNode::Variable(_variable) => {
                self.interpret_variable(typechecker, stmt_id, symbols)?
            }
            CheckedStmtNode::Definition(_definition) => {}
            CheckedStmtNode::Expression(expr_id) => {
                match &typechecker[expr_id.clone()].node_type() {
                    NodeType::BlockExpr => {
                        if let Some(r) = self.interpret_expr(typechecker, *expr_id, symbols)? {
                            return Ok(ControlState::Return(r));
                        };
                    }
                    _ => {
                        self.interpret_expr(typechecker, *expr_id, symbols)?;
                    }
                }
            }
            CheckedStmtNode::Return(_return_node) => {
                return self.interpret_ret(typechecker, stmt_id, symbols);
            }
            CheckedStmtNode::Intrinsic(intrinsic_node) => match intrinsic_node {
                CheckedIntrinsicStmtNode::Assert { left, message } => {
                    let lhs_value = self
                        .interpret_expr(typechecker, left.clone(), symbols)?
                        .unwrap();
                    self.context.assert_true(
                        lhs_value.to_bool(),
                        Box::leak(message.clone().unwrap_or_default().into_boxed_str()),
                    );
                }
                CheckedIntrinsicStmtNode::AssertEq {
                    left,
                    right,
                    message,
                } => {
                    let lhs_value = self
                        .interpret_expr(typechecker, left.clone(), symbols)?
                        .unwrap();
                    let rhs_value = self
                        .interpret_expr(typechecker, right.clone(), symbols)?
                        .unwrap();

                    self.context.assert_eq(
                        lhs_value.to_felt(),
                        rhs_value.to_felt(),
                        Box::leak(message.clone().unwrap_or_default().into_boxed_str()),
                    );
                }
            },
            CheckedStmtNode::Use => {}
        }
        Ok(ControlState::Normal)
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_ret(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        let return_node = typechecker[stmt_id].as_return().unwrap();
        if let Some(expr) = &return_node.ret {
            let value = self.interpret_expr(typechecker, *expr, symbols)?.unwrap();
            return Ok(ControlState::Return(value));
        } else {
            return Ok(ControlState::Normal);
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_value(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        node: &CheckedValueNode<F>,
        symbols: &mut SymbolTable<F>,
    ) -> Result<CheckedValue<F>> {
        Ok(match node {
            CheckedValueNode::Felt(value) => CheckedValue::Felt(*value),
            CheckedValueNode::Bool(value) => CheckedValue::Bool(*value),
            CheckedValueNode::Array(type_id, elements) => {
                let mut values = Vec::new();
                for element in elements {
                    values.push(
                        self.interpret_expr(typechecker, *element, symbols)?
                            .unwrap(),
                    );
                }
                CheckedValue::Array(*type_id, values)
            }
            CheckedValueNode::Struct(type_id, field_values) => {
                let mut values = IndexMap::new();
                for (field_name, field_value) in field_values {
                    values.insert(
                        field_name.clone(),
                        self.interpret_expr(typechecker, *field_value, symbols)?
                            .unwrap(),
                    );
                }
                CheckedValue::Struct(*type_id, values)
            }
            CheckedValueNode::Type(type_id) => CheckedValue::Type(*type_id),
            CheckedValueNode::Tuple(type_id, elements) => {
                let mut values = Vec::new();
                for (elem_type, expr_id) in elements {
                    let value = self
                        .interpret_expr(typechecker, *expr_id, symbols)?
                        .unwrap();

                    values.push((*elem_type, value));
                }
                CheckedValue::Tuple {
                    type_id: *type_id,
                    elements: values,
                }
            }
        })
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_unary(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        unary_node: &CheckedUnaryNode,
        symbols: &mut SymbolTable<F>,
    ) -> Result<CheckedValue<F>> {
        let rhs_value = self
            .interpret_expr(typechecker, unary_node.rhs, symbols)?
            .unwrap();

        Ok(match unary_node.operator {
            UnaryOperator::Neg => todo!(),
            UnaryOperator::Not => {
                if unary_node.type_id == BOOL_TYPE {
                    CheckedValue::Bool(self.context.op_bool_not(rhs_value.to_bool()))
                } else if unary_node.type_id == FELT_TYPE {
                    CheckedValue::Bool(self.context.op_bool_not(rhs_value.to_felt()))
                } else {
                    todo!()
                }
            }
        })
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_binary(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        binary_node: &CheckedBinaryNode,
        symbols: &mut SymbolTable<F>,
    ) -> Result<CheckedValue<F>> {
        use BinaryOperator::*;
        let lhs_value = self
            .interpret_expr(typechecker, binary_node.lhs, symbols)?
            .unwrap();
        let rhs_value = self
            .interpret_expr(typechecker, binary_node.rhs, symbols)?
            .unwrap();

        let value = match binary_node.operator {
            Add => self
                .context
                .op_add(lhs_value.to_felt(), rhs_value.to_felt()),
            Sub => self
                .context
                .op_sub(lhs_value.to_felt(), rhs_value.to_felt()),
            Mul => self
                .context
                .op_mul(lhs_value.to_felt(), rhs_value.to_felt()),
            Div => self
                .context
                .op_div(lhs_value.to_felt(), rhs_value.to_felt()),
            Mod => self
                .context
                .op_mod(lhs_value.to_felt(), rhs_value.to_felt()),
            BitShr => self
                .context
                .op_u32_shr(lhs_value.to_felt(), rhs_value.to_felt()),
            BitShl => self
                .context
                .op_u32_shl(lhs_value.to_felt(), rhs_value.to_felt()),
            BitAnd => self
                .context
                .op_u32_and(lhs_value.to_felt(), rhs_value.to_felt()),
            BitOr => self
                .context
                .op_u32_or(lhs_value.to_felt(), rhs_value.to_felt()),
            BitXor => self
                .context
                .op_u32_xor(lhs_value.to_felt(), rhs_value.to_felt()),
            And => self
                .context
                .op_bool_and(lhs_value.to_bool(), rhs_value.to_bool()),
            Or => self
                .context
                .op_bool_or(lhs_value.to_bool(), rhs_value.to_bool()),
            Eq => {
                if lhs_value.is_felt() {
                    self.context.op_eq(lhs_value.to_felt(), rhs_value.to_felt())
                } else {
                    self.context.op_eq(lhs_value.to_bool(), rhs_value.to_bool())
                }
            }
            Neq => {
                if lhs_value.is_felt() {
                    self.context
                        .op_neq(lhs_value.to_felt(), rhs_value.to_felt())
                } else {
                    self.context
                        .op_neq(lhs_value.to_bool(), rhs_value.to_bool())
                }
            }
            Lt => self.context.op_lt(lhs_value.to_felt(), rhs_value.to_felt()),
            Lte => self
                .context
                .op_lte(lhs_value.to_felt(), rhs_value.to_felt()),
            Gt => self.context.op_gt(lhs_value.to_felt(), rhs_value.to_felt()),
            Gte => self
                .context
                .op_gte(lhs_value.to_felt(), rhs_value.to_felt()),
        };

        if binary_node.type_id == BOOL_TYPE {
            Ok(CheckedValue::Bool(value))
        } else {
            Ok(CheckedValue::Felt(value))
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_assignment_value(
        &mut self,
        _typechecker: &TypeChecker<F, C>,
        old_value: &CheckedValueRef<F>,
        operator: AssignmentOperator,
        value: CheckedValueRef<F>,
        _symbols: &mut SymbolTable<F>,
    ) -> Result<CheckedValueRef<F>> {
        let new_value = match operator {
            AssignmentOperator::Eq => value,
            AssignmentOperator::AddAssign => CheckedValueRef::from_felt(
                self.context.op_add(old_value.to_felt(), value.to_felt()),
            ),
            AssignmentOperator::SubAssign => CheckedValueRef::from_felt(
                self.context.op_sub(old_value.to_felt(), value.to_felt()),
            ),
            AssignmentOperator::MulAssign => CheckedValueRef::from_felt(
                self.context.op_mul(old_value.to_felt(), value.to_felt()),
            ),
            AssignmentOperator::DivAssign => CheckedValueRef::from_felt(
                self.context.op_div(old_value.to_felt(), value.to_felt()),
            ),
            AssignmentOperator::ModAssign => CheckedValueRef::from_felt(
                self.context.op_mod(old_value.to_felt(), value.to_felt()),
            ),
            AssignmentOperator::BitAndAssign => CheckedValueRef::from_felt(
                self.context
                    .op_u32_and(old_value.to_felt(), value.to_felt()),
            ),
            AssignmentOperator::BitOrAssign => CheckedValueRef::from_felt(
                self.context.op_u32_or(old_value.to_felt(), value.to_felt()),
            ),
            AssignmentOperator::BitXorAssign => CheckedValueRef::from_felt(
                self.context
                    .op_u32_xor(old_value.to_felt(), value.to_felt()),
            ),
            AssignmentOperator::BitShlAssign => CheckedValueRef::from_felt(
                self.context
                    .op_u32_shl(old_value.to_felt(), value.to_felt()),
            ),
            AssignmentOperator::BitShrAssign => CheckedValueRef::from_felt(
                self.context
                    .op_u32_shr(old_value.to_felt(), value.to_felt()),
            ),
        };
        self.cset_variable(old_value, &new_value);
        Ok(new_value)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_expr(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        expr_id: ExprId,
        symbols: &mut SymbolTable<F>,
    ) -> Result<Option<CheckedValueRef<F>>> {
        let node = &typechecker[expr_id];
        match node {
            CheckedExprNode::Path(path) => {
                Ok(Some(self.interpret_path(typechecker, path, symbols)?))
            }
            CheckedExprNode::Intrinsic(ctx_node) => Ok(Some({
                match ctx_node {
                    CheckedIntrinsicExprNode::GetUserId { .. } => {
                        CheckedValueRef::from_felt(self.context.get_user_id())
                    }
                    CheckedIntrinsicExprNode::GetContractId { .. } => {
                        CheckedValueRef::from_felt(self.context.get_contract_id())
                    }
                    CheckedIntrinsicExprNode::GetCheckpointId { .. } => CheckedValueRef::new_rc(
                        CheckedValue::Felt(self.context.get_checkpoint_id()),
                    ),
                    CheckedIntrinsicExprNode::GetLastNonce { .. } => {
                        CheckedValueRef::from_felt(self.context.get_last_nonce())
                    }
                    CheckedIntrinsicExprNode::GetUserPublicKeyHash { type_id, .. } => {
                        CheckedValueRef::new_rc(CheckedValue::Array(
                            type_id.clone(),
                            self.context
                                .get_user_public_key_hash()
                                .into_iter()
                                .map(|x| CheckedValueRef::from_felt(x))
                                .collect(),
                        ))
                    }
                    CheckedIntrinsicExprNode::GetStateHashAt {
                        slot_index,
                        type_id,
                    } => {
                        let slot_index = self
                            .interpret_expr(typechecker, slot_index.clone(), symbols)?
                            .unwrap()
                            .to_felt();
                        CheckedValueRef::new_rc(CheckedValue::Array(
                            type_id.clone(),
                            self.context
                                .get_state_hash_at(slot_index)
                                .into_iter()
                                .map(|x| CheckedValueRef::from_felt(x))
                                .collect(),
                        ))
                    }
                    CheckedIntrinsicExprNode::GetOtherContractStateHashAt {
                        contract_state_tree_height,
                        contract_id,
                        slot_index,
                        type_id,
                    } => {
                        let contract_state_tree_height = self
                            .interpret_expr(
                                typechecker,
                                contract_state_tree_height.clone(),
                                symbols,
                            )?
                            .unwrap()
                            .to_felt();
                        let contract_id = self
                            .interpret_expr(typechecker, contract_id.clone(), symbols)?
                            .unwrap()
                            .to_felt();
                        let slot_index = self
                            .interpret_expr(typechecker, slot_index.clone(), symbols)?
                            .unwrap()
                            .to_felt();
                        CheckedValueRef::new_rc(CheckedValue::Array(
                            type_id.clone(),
                            self.context
                                .get_other_contract_state_hash_at(
                                    contract_state_tree_height,
                                    contract_id,
                                    slot_index,
                                )
                                .into_iter()
                                .map(|x| CheckedValueRef::from_felt(x))
                                .collect(),
                        ))
                    }
                    CheckedIntrinsicExprNode::GetOtherUserContractStateHashAt {
                        contract_state_tree_height,
                        user_id,
                        contract_id,
                        slot_index,
                        type_id,
                    } => {
                        let contract_state_tree_height = self
                            .interpret_expr(
                                typechecker,
                                contract_state_tree_height.clone(),
                                symbols,
                            )?
                            .unwrap()
                            .to_felt();
                        let user_id = self
                            .interpret_expr(typechecker, user_id.clone(), symbols)?
                            .unwrap()
                            .to_felt();
                        let contract_id = self
                            .interpret_expr(typechecker, contract_id.clone(), symbols)?
                            .unwrap()
                            .to_felt();
                        let slot_index = self
                            .interpret_expr(typechecker, slot_index.clone(), symbols)?
                            .unwrap()
                            .to_felt();
                        CheckedValueRef::new_rc(CheckedValue::Array(
                            type_id.clone(),
                            self.context
                                .get_other_user_contract_state_hash_at(
                                    contract_state_tree_height,
                                    user_id,
                                    contract_id,
                                    slot_index,
                                )
                                .into_iter()
                                .map(|x| CheckedValueRef::from_felt(x))
                                .collect(),
                        ))
                    }
                    CheckedIntrinsicExprNode::CSetStateHashAt {
                        slot_index,
                        new_value,
                        type_id,
                    } => {
                        let new_value = self
                            .interpret_expr(typechecker, new_value.clone(), symbols)?
                            .unwrap()
                            .to_array();
                        let slot_index = self
                            .interpret_expr(typechecker, slot_index.clone(), symbols)?
                            .unwrap()
                            .to_felt();
                        CheckedValueRef::new_rc(CheckedValue::Array(
                            type_id.clone(),
                            self.context
                                .cset_state_hash_at(slot_index, new_value)
                                .into_iter()
                                .map(|x| CheckedValueRef::from_felt(x))
                                .collect(),
                        ))
                    }
                    CheckedIntrinsicExprNode::Read { offset, .. } => {
                        let contract_id = self.context.get_contract_id();
                        let user_id = self.context.get_user_id();

                        let offset = self
                            .interpret_expr(typechecker, offset.clone(), symbols)?
                            .unwrap();
                        let value = self.context.op_get_state_felt(
                            0,
                            contract_id,
                            user_id,
                            offset.to_felt(),
                        );
                        return Ok(Some(CheckedValueRef::from_felt(value)));
                    }
                    CheckedIntrinsicExprNode::Write { offset, value, .. } => {
                        let offset = self
                            .interpret_expr(typechecker, offset.clone(), symbols)?
                            .unwrap();
                        let value = self
                            .interpret_expr(typechecker, value.clone(), symbols)?
                            .unwrap();
                        return Ok(Some(CheckedValueRef::from_felt(
                            self.context
                                .op_set_state_obj(offset.to_felt(), value.to_felt()),
                        )));
                    }
                    CheckedIntrinsicExprNode::Hash { data, type_id } => {
                        let data = self
                            .interpret_expr(typechecker, data.clone(), symbols)?
                            .unwrap();
                        return Ok(Some(CheckedValueRef::from_vec(
                            type_id.clone(),
                            self.context.hash(&data.to_felts()),
                        )));
                    }
                }
            })),
            CheckedExprNode::Value(value_node) => Ok(Some(CheckedValueRef::new_rc(
                self.interpret_value(typechecker, &value_node, symbols)?,
            ))),
            CheckedExprNode::Binary(binary_node) => Ok(Some(CheckedValueRef::new_rc(
                self.interpret_binary(typechecker, binary_node, symbols)?,
            ))),
            CheckedExprNode::Unary(unary_node) => Ok(Some(CheckedValueRef::new_rc(
                self.interpret_unary(typechecker, unary_node, symbols)?,
            ))),
            CheckedExprNode::Call(call_node) => Ok(self
                .interpret_call(typechecker, call_node, symbols)?
                .unwrap()),
            CheckedExprNode::MemberCall(checked_member_call_node) => Ok(self
                .interpret_member_call(typechecker, checked_member_call_node, symbols)?
                .unwrap()),
            CheckedExprNode::Cast(cast_node) => Ok(Some(CheckedValueRef::new_rc(
                self.interpret_cast(typechecker, cast_node, symbols)?,
            ))),
            CheckedExprNode::IndexAccess(index_access_node) => Ok(Some(
                self.interpret_index_access(typechecker, index_access_node, symbols)?,
            )),
            CheckedExprNode::TupleAccess(tuple_access_node) => Ok(Some(
                self.interpret_tuple_access(typechecker, tuple_access_node, symbols)?,
            )),
            CheckedExprNode::MemberAccess(member_access_node) => Ok(Some(
                self.interpret_member_access(typechecker, member_access_node, symbols)?,
            )),
            CheckedExprNode::BlockExpr(block_expr) => Ok(Some(self.interpret_block_expr(
                typechecker,
                block_expr,
                symbols,
            )?)),
            CheckedExprNode::IfExpr(if_expr) => Ok(Some(self.interpret_if_expr(
                typechecker,
                if_expr,
                symbols,
            )?)),
        }
    }

    fn interpret_member_access(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        member_access_node: &CheckedMemberAccessNode,
        symbols: &mut SymbolTable<F>,
    ) -> Result<CheckedValueRef<F>> {
        let s = self
            .interpret_expr(typechecker, member_access_node.value, symbols)?
            .unwrap();
        assert!(s.is_struct());
        if let Some(value) = s.get_path(&[member_access_node.field.into()]) {
            return Ok(value.clone());
        } else if symbols[member_access_node.type_id]
            .as_function()
            .map(|f| f.name == member_access_node.field)
            .unwrap_or(false)
        {
            return Ok(CheckedValueRef::new_rc(CheckedValue::Type(
                member_access_node.type_id,
            )));
        } else {
            return Err(Error::SemaError(qed_sema::Error::UnresolvedMember));
        }
    }

    fn interpret_index_access(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        index_access_node: &CheckedIndexAccessNode,
        symbols: &mut SymbolTable<F>,
    ) -> Result<CheckedValueRef<F>> {
        let a = self
            .interpret_expr(typechecker, index_access_node.value, symbols)?
            .unwrap();
        return Ok(a.get_path(&[index_access_node.index]).unwrap());
    }

    fn interpret_tuple_access(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        tuple_access_node: &CheckedTupleAccessNode,
        symbols: &mut SymbolTable<F>,
    ) -> Result<CheckedValueRef<F>> {
        // get the value of the tuple
        let tuple_value = self
            .interpret_expr(typechecker, tuple_access_node.value, symbols)?
            .unwrap();

        // store the value of the tuple to avoid lifetime issues
        let tuple_ref = tuple_value.borrow();

        if let CheckedValue::Tuple { type_id: _type_id, elements } = &*tuple_ref {
            // check if the index is out of bounds
            if tuple_access_node.index >= elements.len() {
                return Err(Error::IndexOutOfBounds);
            }

            // get the value of the element
            let (_, element_value) = &elements[tuple_access_node.index];
            return Ok(element_value.clone());
        }

        Err(Error::TypeMismatch)
    }
    fn interpret_cast(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        cast_node: &CheckedCastNode,
        symbols: &mut SymbolTable<F>,
    ) -> Result<CheckedValue<F>> {
        let value = self
            .interpret_expr(typechecker, cast_node.value, symbols)?
            .unwrap();
        if value.is_felt() && cast_node.target_type == BOOL_TYPE {
            return Ok(CheckedValue::Bool(value.to_felt()));
        } else if value.is_bool() && cast_node.target_type == FELT_TYPE {
            return Ok(CheckedValue::Felt(value.to_bool()));
        } else {
            unimplemented!()
        }
    }

    fn interpret_call(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        call_node: &CheckedCallNode,
        symbols: &mut SymbolTable<F>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        let f = self
            .interpret_expr(typechecker, call_node.callee, symbols)?
            .unwrap();
        let mut parameters = Vec::new();
        for arg in call_node.args.iter() {
            parameters.push(
                self.interpret_expr(typechecker, arg.clone(), symbols)?
                    .unwrap(),
            );
        }
        return self.interpret_function(typechecker, f.type_id(), parameters, symbols);
    }

    fn interpret_member_call(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        call_node: &CheckedMemberCallNode,
        symbols: &mut SymbolTable<F>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        let f = self
            .interpret_expr(typechecker, call_node.callee, symbols)?
            .unwrap();
        let mut parameters = Vec::new();
        for arg in once(&call_node.receiver).chain(call_node.args.iter()) {
            parameters.push(
                self.interpret_expr(typechecker, arg.clone(), symbols)?
                    .unwrap(),
            );
        }
        return self.interpret_function(typechecker, f.type_id(), parameters, symbols);
    }

    fn interpret_path(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        path: &CheckedPathNode,
        symbols: &mut SymbolTable<F>,
    ) -> Result<CheckedValueRef<F>> {
        if let Some(variable) = symbols.get_variable(Some(path.scope_id), &path.name) {
            return Ok(variable.value.clone().unwrap());
        } else if let Some(CheckedConstNode { value, .. }) = symbols[path.type_id].as_const() {
            return Ok(self
                .interpret_expr(typechecker, value.clone(), symbols)?
                .unwrap());
        } else {
            return Ok(CheckedValueRef::new_rc(CheckedValue::Type(
                path.type_id.clone(),
            )));
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_assignment(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
    ) -> Result<()> {
        let node = typechecker[stmt_id].as_assignment().unwrap();
        let value = self
            .interpret_expr(typechecker, node.value, symbols)?
            .unwrap();
        let mut path = vec![];

        let (old_value, name, variable) = match &typechecker[node.variable] {
            CheckedExprNode::Path(path_node) => {
                self.interpret_path_assignment(typechecker, node, path_node, symbols, &mut path)?
            }
            CheckedExprNode::MemberAccess(member_access_node) => self.interpret_member_assignment(
                typechecker,
                node,
                member_access_node,
                symbols,
                &mut path,
            )?,
            CheckedExprNode::IndexAccess(index_access_node) => self.interpret_index_assignment(
                typechecker,
                node,
                index_access_node,
                symbols,
                &mut path,
            )?,
            CheckedExprNode::TupleAccess(tuple_access_node) => self.interpret_tuple_assignment(
                typechecker,
                node,
                tuple_access_node,
                symbols,
                &mut path,
            )?,

            _ => unimplemented!(),
        };

        let new_value = self.interpret_assignment_value(
            typechecker,
            &old_value,
            node.operator,
            value,
            symbols,
        )?;

        let mut variable_value = variable.value.unwrap();
        variable_value.set_path(&path, new_value)?;

        symbols.set_variable(variable.scope_id, &name, variable_value)?;

        Ok(())
    }

    fn interpret_index_assignment(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        node: &CheckedAssignmentNode,
        index_access_node: &CheckedIndexAccessNode,
        symbols: &mut SymbolTable<F>,
        path: &mut Vec<usize>,
    ) -> Result<(CheckedValueRef<F>, IdentId, CheckedVariable<F>)> {
        let (inner_value, inner_var_name, inner_var) = match &typechecker[index_access_node.value] {
            CheckedExprNode::Path(checked_path_node) => {
                self.interpret_path_assignment(typechecker, node, checked_path_node, symbols, path)?
            }
            CheckedExprNode::IndexAccess(checked_index_access_node) => self
                .interpret_index_assignment(
                    typechecker,
                    node,
                    checked_index_access_node,
                    symbols,
                    path,
                )?,
            CheckedExprNode::MemberAccess(checked_member_access_node) => self
                .interpret_member_assignment(
                    typechecker,
                    node,
                    checked_member_access_node,
                    symbols,
                    path,
                )?,
            _ => unreachable!(),
        };

        path.push(index_access_node.index);

        assert!(inner_value.is_array());
        Ok((
            inner_value.get_path(&[index_access_node.index]).unwrap(),
            inner_var_name,
            inner_var,
        ))
    }
    fn interpret_tuple_assignment(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        node: &CheckedAssignmentNode,
        tuple_access_node: &CheckedTupleAccessNode,
        symbols: &mut SymbolTable<F>,
        path: &mut Vec<usize>,
    ) -> Result<(CheckedValueRef<F>, IdentId, CheckedVariable<F>)> {
        // get the value of the tuple recursively
        let (tuple_value, tuple_var_name, tuple_var) = match &typechecker[tuple_access_node.value] {
            CheckedExprNode::Path(checked_path_node) => {
                self.interpret_path_assignment(typechecker, node, checked_path_node, symbols, path)?
            }
            CheckedExprNode::IndexAccess(checked_index_access_node) => self
                .interpret_index_assignment(
                    typechecker,
                    node,
                    checked_index_access_node,
                    symbols,
                    path,
                )?,
            CheckedExprNode::MemberAccess(checked_member_access_node) => self
                .interpret_member_assignment(
                    typechecker,
                    node,
                    checked_member_access_node,
                    symbols,
                    path,
                )?,
            CheckedExprNode::TupleAccess(checked_tuple_access_node) => self
                .interpret_tuple_assignment(
                    typechecker,
                    node,
                    checked_tuple_access_node,
                    symbols,
                    path,
                )?,
            _ => unreachable!(),
        };

        // ensure that the value is a tuple
        assert!(
            tuple_value.is_tuple(),
            "Expected tuple, found {:?}",
            tuple_value
        );

        // push the index of the tuple element to the path
        path.push(tuple_access_node.index);

        // visit the tuple element and return it
        let element_value = tuple_value.get_path(&[tuple_access_node.index]).unwrap();

        Ok((element_value, tuple_var_name, tuple_var))
    }
    fn interpret_member_assignment(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        node: &CheckedAssignmentNode,
        member_access_node: &CheckedMemberAccessNode,
        symbols: &mut SymbolTable<F>,
        path: &mut Vec<usize>,
    ) -> Result<(CheckedValueRef<F>, IdentId, CheckedVariable<F>)> {
        let (inner_value, inner_var_name, inner_var) = match &typechecker[member_access_node.value]
        {
            CheckedExprNode::Path(checked_path_node) => {
                self.interpret_path_assignment(typechecker, node, checked_path_node, symbols, path)?
            }
            CheckedExprNode::IndexAccess(checked_index_access_node) => self
                .interpret_index_assignment(
                    typechecker,
                    node,
                    checked_index_access_node,
                    symbols,
                    path,
                )?,
            CheckedExprNode::MemberAccess(checked_member_access_node) => self
                .interpret_member_assignment(
                    typechecker,
                    node,
                    checked_member_access_node,
                    symbols,
                    path,
                )?,
            _ => unreachable!(),
        };

        path.push(member_access_node.field.into());

        assert!(inner_value.is_struct());
        Ok((
            inner_value
                .get_path(&[member_access_node.field.into()])
                .unwrap(),
            inner_var_name,
            inner_var,
        ))
    }

    fn interpret_path_assignment(
        &mut self,
        _typechecker: &TypeChecker<F, C>,
        _node: &CheckedAssignmentNode,
        path_node: &CheckedPathNode,
        symbols: &mut SymbolTable<F>,
        _path: &mut Vec<usize>,
    ) -> Result<(CheckedValueRef<F>, IdentId, CheckedVariable<F>)> {
        let start_scope = Some(path_node.scope_id);
        let variable = symbols.get_variable(start_scope, &path_node.name).unwrap();
        Ok((variable.value.clone().unwrap(), path_node.name, variable))
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_variable(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
    ) -> Result<()> {
        let node = typechecker[stmt_id].as_variable().unwrap();
        let value = match self.interpret_expr(typechecker, node.value, symbols)? {
            Some(value) => value,
            None => {
                return Err(Error::SemaError(qed_sema::Error::UnresolvedVariable));
            }
        };

        symbols.set_variable(node.scope_id, &node.name, value.clone())?;
        Ok(())
    }

    fn cset_variable(&mut self, old_value: &CheckedValueRef<F>, new_value: &CheckedValueRef<F>) {
        if old_value == new_value {
            return;
        }

        let mut new_value_mut = new_value.borrow_mut();
        match (&*old_value.borrow(), &mut *new_value_mut) {
            (CheckedValue::Felt(o), CheckedValue::Felt(ref mut n)) => {
                *n = self.context.cset(o.clone(), n.clone());
            }
            (CheckedValue::Bool(o), CheckedValue::Bool(ref mut n)) => {
                *n = self.context.cset(o.clone(), n.clone());
            }
            (CheckedValue::Array(lhs_type_id, o), CheckedValue::Array(rhs_type_id, n))
                if lhs_type_id == rhs_type_id =>
            {
                for (old_value, new_value) in o.iter().zip(n.iter()) {
                    self.cset_variable(old_value, new_value);
                }
            }
            (CheckedValue::Struct(lhs_type_id, o), CheckedValue::Struct(rhs_type_id, n))
                if lhs_type_id == rhs_type_id =>
            {
                for ((old_field_name, old_field_value), (new_field_name, new_field_value)) in
                    o.iter().zip(n.iter())
                {
                    assert_eq!(old_field_name, new_field_name);
                    self.cset_variable(old_field_value, new_field_value);
                }
            }
            (
                CheckedValue::Tuple {
                    type_id: lhs_tid,
                    elements: old_elements,
                },
                CheckedValue::Tuple {
                    type_id: rhs_tid,
                    elements: new_elements,
                },
            ) if lhs_tid == rhs_tid => {
                assert_eq!(
                    old_elements.len(),
                    new_elements.len(),
                    "Tuple size mismatch"
                );

                for ((old_type_id, old_value), (new_type_id, new_value)) in
                    old_elements.iter().zip(new_elements.iter_mut())
                {
                    assert_eq!(old_type_id, new_type_id, "Tuple element type mismatch");
                    self.cset_variable(old_value, new_value);
                }
            }

            _ => {
                unreachable!()
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_block_expr(
        &mut self,
        type_checker: &TypeChecker<F, C>,
        block_expr: &CheckedBlockExprNode,
        symbols: &mut SymbolTable<F>,
    ) -> Result<CheckedValueRef<F>> {
        let void_value: CheckedValueRef<F> = CheckedValueRef::new_rc(CheckedValue::Type(VOID_TYPE));

        for stmt_id in &block_expr.stmts {
            match self.interpret_statement(type_checker, *stmt_id, symbols)? {
                ControlState::Return(value) => {
                    return Ok(value);
                }
                ControlState::Normal => {}
            }
        }
        Ok(void_value)
    }
    #[instrument(level = "debug", skip_all)]
    pub fn interpret_if_expr(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        if_expr: &CheckedIfExprNode,
        symbols: &mut SymbolTable<F>,
    ) -> Result<CheckedValueRef<F>> {
        let node = if_expr;
        let mut result: CheckedValueRef<F>;

        //calculate the predicate
        let predicate = self
            .interpret_expr(typechecker, node.if_branch.predicate, symbols)?
            .unwrap()
            .to_bool();

        //enter the if block
        self.context.start_if_block(predicate);

        //use if block return value to initialize the result
        result =
            match self.interpret_statement(typechecker, node.if_branch.body.clone(), symbols)? {
                ControlState::Return(value) => value,
                //todo!: change type to CheckedValue::Type(Void)
                ControlState::Normal => CheckedValueRef::new_rc(CheckedValue::Type(VOID_TYPE)),
            };

        for condition in &node.elseif_branches {
            let predicate = self
                .interpret_expr(typechecker, condition.predicate, symbols)?
                .unwrap()
                .to_bool();

            self.context.start_else_if_block(predicate);
            let elseif_result =
                match self.interpret_statement(typechecker, condition.body.clone(), symbols)? {
                    ControlState::Return(value) => value,
                    ControlState::Normal => CheckedValueRef::new_rc(CheckedValue::Type(VOID_TYPE)),
                };
            result = self.context.cset(result, elseif_result);
        }

        if let Some(else_branch) = &node.else_branch {
            self.context.start_else_block();

            let else_result =
                match self.interpret_statement(typechecker, else_branch.clone(), symbols)? {
                    ControlState::Return(value) => value,
                    ControlState::Normal => CheckedValueRef::new_rc(CheckedValue::Type(VOID_TYPE)),
                };
            result = self.context.cset(result, else_result);
        }

        self.context.end_if_block();

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};
    use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
    use qed_core::{config::network_constants::GLOBAL_USER_TREE_HEIGHT, data::qhashout::QHashOut};
    use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
    use qed_data::qblock::cmds::register_user::QBCRegisterUser;
    use qed_exec::vm::exec::QEDEvalSessionResult;

    use qed_store::config::store_config::QEDHasher;
    use qed_utils::{
        gen_contract_deploy_and_circuits_for_functions, prepare_environment_with_real_contract, C,
        D,
    };
    use qedlang_core::dpn::{
        ops::{exec_context::QExecContext, sym_felt::SymFeltRef},
        vm::compile::QEDCompileResult,
    };

    use super::*;

    #[test]
    fn test_interpreter() {
        qed_utils::setup_env_logger();

        insta::glob!("../../tests", "00*.qed", |path| {
            let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());

            let compile_results = interpreter
                .interpret(
                    path.into(),
                    None,
                    vec!["main"],
                    |context, (method_name, method_id, outputs)| {
                        QEDCompileResult::compile_exec(
                            method_name,
                            method_id,
                            &context.store,
                            &context,
                            &outputs,
                        )
                    },
                )
                .unwrap();

            let priv_key = QHashOut::rand();
            let wallet = SimpleQEDZKSignatureManager::<C, D>::new();
            let priv_key_w = SimpleQEDPrivateKey::new(priv_key);
            let pub_key_param = priv_key_w.get_public_key_param::<QEDHasher>();
            let contract_state_tree_height = GLOBAL_USER_TREE_HEIGHT as usize;

            let deployer = QHashOut::rand();
            let (_circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions(
                deployer,
                contract_state_tree_height as u8,
                &compile_results,
            )
            .unwrap();

            let mut lps = prepare_environment_with_real_contract(
                QBCRegisterUser::new(wallet.get_zksig_circuit_fingerprint(), pub_key_param),
                deploy_cmd,
            )
            .unwrap();
            let contract_id = GoldilocksField::from_canonical_u64(2);

            let cfc_input = QEDEvalSessionResult::new()
                .exec_contract_call(&mut lps, contract_id, &compile_results[0], vec![])
                .unwrap();
            println!("result_vm: {:?}", cfc_input.outputs);
            unsafe { STD_PRIMITIVE_SCOPE_ID.take().unwrap() };
        });
    }
}
