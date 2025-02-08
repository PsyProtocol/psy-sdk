#![feature(try_trait_v2)]

mod control;
mod error;
mod preprocess;

use either::Either;
use error::{Error, Result};
use indexmap::IndexMap;
pub use preprocess::StorageProcessor;
use qed_ast::*;
use qed_builder::{ContextFelt, ContextInput, DPNContext};
use qed_fmt::Formatter;
use qed_parser::Parser;
use qed_sema::Error as SemaError;
use qed_sema::*;
use std::{cell::RefCell, collections::HashMap, fmt::Display, ops::Index, path::PathBuf, rc::Rc};

use tracing::{debug, error, info, instrument, span, Level};

use crate::control::ControlState;

#[derive(Debug)]
pub struct Interpreter<F: Clone + From<u32>, C> {
    pub inputs: Vec<u64>,
    pub context: C,
    _marker: std::marker::PhantomData<F>,
}

impl<F: ContextFelt + From<u32>, C: DPNContext<F>> ContextInput for Interpreter<F, C> {
    fn get_input(&self, index: u64) -> u64 {
        self.inputs[index as usize]
    }
}

impl<F: ContextFelt + From<u32>, C: DPNContext<F>> Interpreter<F, C> {
    pub fn new(context: C) -> Self {
        Self {
            inputs: vec![],
            context,
            _marker: std::marker::PhantomData,
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn compile(
        &mut self,
        typechecker: &mut TypeChecker<F, C>,
        entry: PathBuf,
    ) -> Result<Option<CheckedValueRef<F>>>
    where
        F: Display + 'static,
    {
        let mut program = Program::new();
        let mut parser = Parser::new(&mut program);
        parser
            .parse(&mut self.context, entry)
            .map_err(|err| Error::ParseError(err.to_string()))?;

        let mut storage_preprocessor: StorageProcessor = StorageProcessor::new();
        let mut default_visitor_context: DefaultVisitorContext<'_, F, C> =
            DefaultVisitorContext::new(&mut program);
        storage_preprocessor.visit_program(&mut default_visitor_context);

        let mut formatter = Formatter::new();
        formatter.visit_program(&mut default_visitor_context);
        println!("formatted:\n{}", formatter.get_output());
        // println!("ast:\n{:#?}", program);

        let mut typechecker_context = TypeCheckerVisitorContext::new(program);
        typechecker.visit_program(&mut typechecker_context)?;
        let scope_id = typechecker_context.symbols[ModuleId::root()].scope_id;
        let type_id = typechecker_context.symbols[scope_id]
            .types
            .get(&IdentId::MAIN.into())
            .ok_or(Error::UndefinedMain)?
            .clone();
        let node: CheckedFunctionNode =
            (typechecker_context.symbols[type_id.clone()].as_ref() as &CheckedFunctionNode).clone();

        let mut parameters = vec![];
        for (id, _, ty) in node.parameters.iter() {
            parameters.push(CheckedValueRef::new_rc(
                typechecker_context.symbols[ty.clone()]
                    .clone()
                    .to_value(&mut typechecker_context.symbols, &mut self.context),
            ));
        }

        self.interpret_function(
            typechecker,
            type_id,
            parameters,
            &mut typechecker_context.symbols,
            Some(NodeType::Module),
        )
        .unwrap()
        .transpose()
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret(
        &mut self,
        typechecker: &mut TypeChecker<F, C>,
        entry: PathBuf,
        parameters: Vec<CheckedValue<F>>,
    ) -> Result<Option<CheckedValueRef<F>>>
    where
        F: Display + 'static,
    {
        let mut program = Program::new();
        let mut parser = Parser::new(&mut program);
        parser
            .parse(&mut self.context, entry)
            .map_err(|err| Error::ParseError(err.to_string()))?;

        let mut storage_preprocessor: StorageProcessor = StorageProcessor::new();
        let mut default_visitor_context: DefaultVisitorContext<'_, F, C> =
            DefaultVisitorContext::new(&mut program);
        storage_preprocessor.visit_program(&mut default_visitor_context);

        let mut formatter = Formatter::new();
        formatter.visit_program(&mut default_visitor_context);
        println!("formatted:\n{}", formatter.get_output());
        // println!("ast:\n{:#?}", program);

        let mut typechecker_context = TypeCheckerVisitorContext::new(program);
        typechecker.visit_program(&mut typechecker_context)?;
        let scope_id = typechecker_context.symbols[ModuleId::root()].scope_id;
        let type_id = typechecker_context.symbols[scope_id]
            .types
            .get(&IdentId::MAIN.into())
            .ok_or(Error::UndefinedMain)?;

        self.interpret_function(
            typechecker,
            *type_id,
            parameters
                .into_iter()
                .map(|x| CheckedValueRef::new_rc(x))
                .collect(),
            &mut typechecker_context.symbols,
            Some(NodeType::Module),
        )
        .unwrap()
        .transpose()
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_function(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        type_id: TypeId,
        parameters: Vec<CheckedValueRef<F>>,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> ControlState<Result<CheckedValueRef<F>>> {
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
    ) -> ControlState<Result<CheckedValueRef<F>>> {
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

        self.interpret_block(
            typechecker,
            node.body.unwrap(),
            symbols,
            Some(node.node_type()),
        )
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_if(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> Result<()> {
        let node = typechecker[stmt_id].as_if().unwrap();
        let predicate = self
            .interpret_expr(
                typechecker,
                node.if_branch.predicate,
                symbols,
                Some(node.node_type()),
            )?
            .unwrap()
            .to_bool();
        self.context.start_if_block(predicate);
        self.interpret_block(
            typechecker,
            node.if_branch.body,
            symbols,
            Some(node.node_type()),
        );

        for condition in &node.elseif_branch {
            let predicate = self
                .interpret_expr(
                    typechecker,
                    condition.predicate,
                    symbols,
                    Some(node.node_type()),
                )?
                .unwrap()
                .to_bool();
            self.context.start_else_if_block(predicate);
            self.interpret_block(typechecker, condition.body, symbols, Some(node.node_type()));
        }

        if let Some(else_branch) = &node.else_branch {
            self.context.start_else_block();
            self.interpret_block(
                typechecker,
                else_branch.clone(),
                symbols,
                Some(node.node_type()),
            );
        }

        self.context.end_if_block();
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_while(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> Result<()> {
        let node = typechecker[stmt_id].as_while().unwrap();
        loop {
            let predicate = self
                .interpret_expr(typechecker, node.predicate, symbols, Some(node.node_type()))?
                .unwrap()
                .to_bool();
            if self.context.get_bool_value(predicate) {
                self.context.start_if_block(predicate);
                self.interpret_block(typechecker, node.body, symbols, Some(node.node_type()));
                self.context.end_if_block();
            } else {
                break Ok(());
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_block(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> ControlState<Result<CheckedValueRef<F>>> {
        let node = typechecker[stmt_id].as_block().unwrap();
        for &stmt in &node.stmts {
            self.interpret_statement(typechecker, stmt, symbols, Some(node.node_type()))?;
        }
        ControlState::Normal
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_statement(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> ControlState<Result<CheckedValueRef<F>>> {
        let node = &typechecker[stmt_id];
        match node {
            CheckedStmtNode::If(r#if) => {
                self.interpret_if(typechecker, stmt_id, symbols, parent_node_type)?
            }
            CheckedStmtNode::While(r#while) => {
                self.interpret_while(typechecker, stmt_id, symbols, parent_node_type)?
            }
            CheckedStmtNode::Block(block) => {
                return self.interpret_block(typechecker, stmt_id, symbols, parent_node_type);
            }
            CheckedStmtNode::Assignment(r#assignment) => {
                self.interpret_assignment(typechecker, stmt_id, symbols, parent_node_type)?
            }
            CheckedStmtNode::Variable(variable) => {
                self.interpret_variable(typechecker, stmt_id, symbols, parent_node_type)?
            }
            CheckedStmtNode::Definition(definition) => {}
            CheckedStmtNode::Expression(expr_id) => {
                self.interpret_expr(typechecker, *expr_id, symbols, parent_node_type)?;
            }
            CheckedStmtNode::Return(return_node) => {
                return self.interpret_ret(typechecker, stmt_id, symbols, parent_node_type);
            }
            CheckedStmtNode::Storage(storage) => {
                self.interpret_storage_write(typechecker, stmt_id, symbols, parent_node_type)?;
            }
        }
        ControlState::Normal
    }

    fn interpret_storage_write(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> ControlState<Result<CheckedValueRef<F>>> {
        let storage = typechecker[stmt_id].as_storage().unwrap();
        let offset = self
            .interpret_expr(
                typechecker,
                storage.offset,
                symbols,
                Some(storage.node_type()),
            )?
            .unwrap();
        let value = self
            .interpret_expr(
                typechecker,
                storage.value,
                symbols,
                Some(storage.node_type()),
            )?
            .unwrap();
        self.context
            .op_set_state_felt(offset.to_felt(), value.to_felt());
        return ControlState::Normal;
    }

    fn interpret_ret(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> ControlState<Result<CheckedValueRef<F>>> {
        let return_node = typechecker[stmt_id].as_return().unwrap();
        if let Some((expr, _)) = &return_node.ret {
            let value = self
                .interpret_expr(typechecker, *expr, symbols, Some(return_node.node_type()))?
                .unwrap();
            return ControlState::Return(Ok(value));
        } else {
            return ControlState::Normal;
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_value(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        node: &CheckedValueNode<F>,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        Ok(match node {
            CheckedValueNode::Felt(value) => CheckedValue::Felt(*value),
            CheckedValueNode::Bool(value) => CheckedValue::Bool(*value),
            CheckedValueNode::Array(type_id, elements) => {
                let mut values = Vec::new();
                for element in elements {
                    values.push(
                        self.interpret_expr(
                            typechecker,
                            *element,
                            symbols,
                            Some(node.node_type()),
                        )?
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
                        self.interpret_expr(
                            typechecker,
                            *field_value,
                            symbols,
                            Some(node.node_type()),
                        )?
                        .unwrap(),
                    );
                }
                CheckedValue::Struct(*type_id, values)
            }
            CheckedValueNode::Type(type_id) => CheckedValue::Type(*type_id),
        })
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_unary(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        unary_node: &CheckedUnaryNode,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        let rhs_value = self
            .interpret_expr(
                typechecker,
                unary_node.rhs,
                symbols,
                Some(unary_node.node_type()),
            )?
            .unwrap();

        Ok(match unary_node.operator {
            UnaryOperator::Neg => CheckedValue::Felt(self.context.op_neg(rhs_value.to_felt())),
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
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        use BinaryOperator::*;
        let lhs_value = self
            .interpret_expr(
                typechecker,
                binary_node.lhs,
                symbols,
                Some(binary_node.node_type()),
            )?
            .unwrap();
        let rhs_value = self
            .interpret_expr(
                typechecker,
                binary_node.rhs,
                symbols,
                Some(binary_node.node_type()),
            )?
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
        typechecker: &TypeChecker<F, C>,
        old_value: &CheckedValueRef<F>,
        operator: AssignmentOperator,
        value: CheckedValueRef<F>,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValueRef<F>> {
        let new_value = match operator {
            AssignmentOperator::Eq => value,
            AssignmentOperator::AddAssign => CheckedValueRef::new_rc(CheckedValue::Felt(
                self.context.op_add(old_value.to_felt(), value.to_felt()),
            )),
            AssignmentOperator::SubAssign => CheckedValueRef::new_rc(CheckedValue::Felt(
                self.context.op_sub(old_value.to_felt(), value.to_felt()),
            )),
            AssignmentOperator::MulAssign => CheckedValueRef::new_rc(CheckedValue::Felt(
                self.context.op_mul(old_value.to_felt(), value.to_felt()),
            )),
            AssignmentOperator::DivAssign => CheckedValueRef::new_rc(CheckedValue::Felt(
                self.context.op_div(old_value.to_felt(), value.to_felt()),
            )),
            AssignmentOperator::ModAssign => CheckedValueRef::new_rc(CheckedValue::Felt(
                self.context.op_mod(old_value.to_felt(), value.to_felt()),
            )),
            AssignmentOperator::BitAndAssign => CheckedValueRef::new_rc(CheckedValue::Felt(
                self.context
                    .op_u32_and(old_value.to_felt(), value.to_felt()),
            )),
            AssignmentOperator::BitOrAssign => CheckedValueRef::new_rc(CheckedValue::Felt(
                self.context.op_u32_or(old_value.to_felt(), value.to_felt()),
            )),
            AssignmentOperator::BitXorAssign => CheckedValueRef::new_rc(CheckedValue::Felt(
                self.context
                    .op_u32_xor(old_value.to_felt(), value.to_felt()),
            )),
            AssignmentOperator::BitShlAssign => CheckedValueRef::new_rc(CheckedValue::Felt(
                self.context
                    .op_u32_shl(old_value.to_felt(), value.to_felt()),
            )),
            AssignmentOperator::BitShrAssign => CheckedValueRef::new_rc(CheckedValue::Felt(
                self.context
                    .op_u32_shr(old_value.to_felt(), value.to_felt()),
            )),
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
        parent_node_type: Option<NodeType>,
    ) -> Result<Option<CheckedValueRef<F>>> {
        let node = &typechecker[expr_id];
        match node {
            CheckedExprNode::Path(path) => Ok(Some(self.interpret_path(
                typechecker,
                path,
                symbols,
                parent_node_type,
            )?)),
            CheckedExprNode::Storage(storage_read) => Ok(Some(self.interpret_storage_read(
                typechecker,
                storage_read,
                symbols,
                parent_node_type,
            )?)),
            CheckedExprNode::Context(ctx_node) => Ok(Some({
                match ctx_node {
                    CheckedContextNode::GetUserId { .. } => {
                        CheckedValueRef::new_rc(CheckedValue::Felt(self.context.get_user_id()))
                    }
                    CheckedContextNode::GetContractId { .. } => {
                        CheckedValueRef::new_rc(CheckedValue::Felt(self.context.get_contract_id()))
                    }
                    CheckedContextNode::GetCheckpointId { .. } => CheckedValueRef::new_rc(
                        CheckedValue::Felt(self.context.get_checkpoint_id()),
                    ),
                    CheckedContextNode::GetLastNonce { .. } => {
                        CheckedValueRef::new_rc(CheckedValue::Felt(self.context.get_last_nonce()))
                    }
                    CheckedContextNode::GetUserPublicKeyHash { type_id, .. } => {
                        CheckedValueRef::new_rc(CheckedValue::Array(
                            type_id.clone(),
                            self.context
                                .get_user_public_key_hash()
                                .into_iter()
                                .map(|x| CheckedValueRef::new_rc(CheckedValue::Felt(x)))
                                .collect(),
                        ))
                    }
                    CheckedContextNode::GetStateHashAt {
                        slot_index,
                        type_id,
                    } => {
                        let slot_index = self
                            .interpret_expr(
                                typechecker,
                                slot_index.clone(),
                                symbols,
                                Some(ctx_node.node_type()),
                            )?
                            .unwrap()
                            .to_felt();
                        CheckedValueRef::new_rc(CheckedValue::Array(
                            type_id.clone(),
                            self.context
                                .get_state_hash_at(slot_index)
                                .into_iter()
                                .map(|x| CheckedValueRef::new_rc(CheckedValue::Felt(x)))
                                .collect(),
                        ))
                    }
                    CheckedContextNode::GetOtherContractStateHashAt {
                        contract_state_tree_height,
                        contract_id,
                        slot_index,
                        type_id,
                    } => {
                        let parent_node_type = ctx_node.node_type();
                        let contract_state_tree_height = self
                            .interpret_expr(
                                typechecker,
                                contract_state_tree_height.clone(),
                                symbols,
                                Some(parent_node_type),
                            )?
                            .unwrap()
                            .to_felt();
                        let contract_id = self
                            .interpret_expr(
                                typechecker,
                                contract_id.clone(),
                                symbols,
                                Some(parent_node_type),
                            )?
                            .unwrap()
                            .to_felt();
                        let slot_index = self
                            .interpret_expr(
                                typechecker,
                                slot_index.clone(),
                                symbols,
                                Some(parent_node_type),
                            )?
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
                                .map(|x| CheckedValueRef::new_rc(CheckedValue::Felt(x)))
                                .collect(),
                        ))
                    }
                    CheckedContextNode::GetOtherUserContractStateHashAt {
                        contract_state_tree_height,
                        user_id,
                        contract_id,
                        slot_index,
                        type_id,
                    } => {
                        let parent_node_type = ctx_node.node_type();
                        let contract_state_tree_height = self
                            .interpret_expr(
                                typechecker,
                                contract_state_tree_height.clone(),
                                symbols,
                                Some(parent_node_type),
                            )?
                            .unwrap()
                            .to_felt();
                        let user_id = self
                            .interpret_expr(
                                typechecker,
                                user_id.clone(),
                                symbols,
                                Some(parent_node_type),
                            )?
                            .unwrap()
                            .to_felt();
                        let contract_id = self
                            .interpret_expr(
                                typechecker,
                                contract_id.clone(),
                                symbols,
                                Some(parent_node_type),
                            )?
                            .unwrap()
                            .to_felt();
                        let slot_index = self
                            .interpret_expr(
                                typechecker,
                                slot_index.clone(),
                                symbols,
                                Some(parent_node_type),
                            )?
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
                                .map(|x| CheckedValueRef::new_rc(CheckedValue::Felt(x)))
                                .collect(),
                        ))
                    }
                    CheckedContextNode::CSetStateHashAt {
                        slot_index,
                        new_value,
                        type_id,
                    } => {
                        let parent_node_type = ctx_node.node_type();
                        let new_value = self
                            .interpret_expr(
                                typechecker,
                                new_value.clone(),
                                symbols,
                                Some(parent_node_type),
                            )?
                            .unwrap()
                            .to_array();
                        let slot_index = self
                            .interpret_expr(
                                typechecker,
                                slot_index.clone(),
                                symbols,
                                Some(parent_node_type),
                            )?
                            .unwrap()
                            .to_felt();
                        CheckedValueRef::new_rc(CheckedValue::Array(
                            type_id.clone(),
                            self.context
                                .cset_state_hash_at(slot_index, new_value)
                                .into_iter()
                                .map(|x| CheckedValueRef::new_rc(CheckedValue::Felt(x)))
                                .collect(),
                        ))
                    }
                }
            })),
            CheckedExprNode::Value(value_node) => Ok(Some(CheckedValueRef::new_rc(
                self.interpret_value(typechecker, &value_node, symbols, parent_node_type)?,
            ))),
            CheckedExprNode::Binary(binary_node) => Ok(Some(CheckedValueRef::new_rc(
                self.interpret_binary(typechecker, binary_node, symbols, parent_node_type)?,
            ))),
            CheckedExprNode::Unary(unary_node) => Ok(Some(CheckedValueRef::new_rc(
                self.interpret_unary(typechecker, unary_node, symbols, parent_node_type)?,
            ))),
            CheckedExprNode::Call(call_node) => Ok(self
                .interpret_call(typechecker, call_node, symbols, parent_node_type)
                .unwrap()
                .transpose()?),
            CheckedExprNode::Cast(cast_node) => Ok(Some(CheckedValueRef::new_rc(
                self.interpret_cast(typechecker, cast_node, symbols, parent_node_type)?,
            ))),
            CheckedExprNode::IndexAccess(index_access_node) => {
                Ok(Some(self.interpret_index_access(
                    typechecker,
                    index_access_node,
                    symbols,
                    parent_node_type,
                )?))
            }
            CheckedExprNode::MemberAccess(member_access_node) => {
                Ok(Some(self.interpret_member_access(
                    typechecker,
                    member_access_node,
                    symbols,
                    parent_node_type,
                )?))
            }
        }
    }

    fn interpret_member_access(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        member_access_node: &CheckedMemberAccessNode,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValueRef<F>> {
        let s = self
            .interpret_expr(
                typechecker,
                member_access_node.value,
                symbols,
                Some(member_access_node.node_type()),
            )?
            .unwrap();
        assert!(s.is_struct());
        Ok(
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
            },
        )
    }

    fn interpret_index_access(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        index_access_node: &CheckedIndexAccessNode,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValueRef<F>> {
        let a = self
            .interpret_expr(
                typechecker,
                index_access_node.value,
                symbols,
                Some(index_access_node.node_type()),
            )?
            .unwrap();
        return Ok(a.get_path(&[index_access_node.index]).unwrap());
    }

    fn interpret_cast(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        cast_node: &CheckedCastNode,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        let value = self
            .interpret_expr(
                typechecker,
                cast_node.value,
                symbols,
                Some(cast_node.node_type()),
            )?
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
        parent_node_type: Option<NodeType>,
    ) -> ControlState<Result<CheckedValueRef<F>>> {
        let f = self
            .interpret_expr(
                typechecker,
                call_node.variable,
                symbols,
                Some(call_node.node_type()),
            )?
            .unwrap();
        let mut parameters = Vec::new();
        for arg in call_node.receiver.iter().chain(call_node.args.iter()) {
            parameters.push(
                self.interpret_expr(
                    typechecker,
                    arg.clone(),
                    symbols,
                    Some(call_node.node_type()),
                )?
                .unwrap(),
            );
        }
        return self.interpret_function(
            typechecker,
            f.type_id(),
            parameters,
            symbols,
            Some(call_node.node_type()),
        );
    }

    fn interpret_path(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        path: &CheckedPathNode,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValueRef<F>> {
        if let Some(variable) = symbols.get_variable(Some(path.scope_id), &path.name) {
            return Ok(variable.value.clone().unwrap());
        } else {
            return Ok(CheckedValueRef::new_rc(CheckedValue::Type(
                path.type_id.clone(),
            )));
        }
    }

    fn interpret_storage_read(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        storage_read: &CheckedStorageReadNode,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValueRef<F>> {
        let contract_id = self.context.get_contract_id();
        let user_id = self.context.get_user_id();

        let offset = self
            .interpret_expr(
                typechecker,
                storage_read.offset,
                symbols,
                Some(storage_read.node_type()),
            )?
            .unwrap();
        let value = self
            .context
            .op_get_state_felt(0, contract_id, user_id, offset.to_felt());
        return Ok(CheckedValueRef::new_rc(CheckedValue::Felt(value)));
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_assignment(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        stmt_id: StmtId,
        symbols: &mut SymbolTable<F>,
        parent_node_type: Option<NodeType>,
    ) -> Result<()> {
        let node = typechecker[stmt_id].as_assignment().unwrap();
        let value = self
            .interpret_expr(typechecker, node.value, symbols, Some(node.node_type()))?
            .unwrap();
        let mut path = vec![];

        let (old_value, name, variable) = match &typechecker[node.variable] {
            CheckedExprNode::Path(path_node) => self.interpret_path_assignment(
                typechecker,
                node,
                path_node,
                symbols,
                &mut path,
                parent_node_type,
            )?,
            CheckedExprNode::MemberAccess(member_access_node) => self.interpret_member_assignment(
                typechecker,
                node,
                member_access_node,
                symbols,
                &mut path,
                parent_node_type,
            )?,
            CheckedExprNode::IndexAccess(index_access_node) => self.interpret_index_assignment(
                typechecker,
                node,
                index_access_node,
                symbols,
                &mut path,
                parent_node_type,
            )?,
            _ => unimplemented!(),
        };

        let new_value = self.interpret_assignment_value(
            typechecker,
            &old_value,
            node.operator,
            value,
            symbols,
            parent_node_type,
        )?;

        let mut variable_value = variable.value.unwrap();
        variable_value.set_path(&path, new_value);

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
        parent_node_type: Option<NodeType>,
    ) -> Result<(CheckedValueRef<F>, IdentId, CheckedVariable<F>)> {
        let (inner_value, inner_var_name, inner_var) = match &typechecker[index_access_node.value] {
            CheckedExprNode::Path(checked_path_node) => self.interpret_path_assignment(
                typechecker,
                node,
                checked_path_node,
                symbols,
                path,
                parent_node_type,
            )?,
            CheckedExprNode::IndexAccess(checked_index_access_node) => self
                .interpret_index_assignment(
                    typechecker,
                    node,
                    checked_index_access_node,
                    symbols,
                    path,
                    parent_node_type,
                )?,
            CheckedExprNode::MemberAccess(checked_member_access_node) => self
                .interpret_member_assignment(
                    typechecker,
                    node,
                    checked_member_access_node,
                    symbols,
                    path,
                    parent_node_type,
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

    fn interpret_member_assignment(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        node: &CheckedAssignmentNode,
        member_access_node: &CheckedMemberAccessNode,
        symbols: &mut SymbolTable<F>,
        path: &mut Vec<usize>,
        parent_node_type: Option<NodeType>,
    ) -> Result<(CheckedValueRef<F>, IdentId, CheckedVariable<F>)> {
        let (inner_value, inner_var_name, inner_var) = match &typechecker[member_access_node.value]
        {
            CheckedExprNode::Path(checked_path_node) => self.interpret_path_assignment(
                typechecker,
                node,
                checked_path_node,
                symbols,
                path,
                parent_node_type,
            )?,
            CheckedExprNode::IndexAccess(checked_index_access_node) => self
                .interpret_index_assignment(
                    typechecker,
                    node,
                    checked_index_access_node,
                    symbols,
                    path,
                    parent_node_type,
                )?,
            CheckedExprNode::MemberAccess(checked_member_access_node) => self
                .interpret_member_assignment(
                    typechecker,
                    node,
                    checked_member_access_node,
                    symbols,
                    path,
                    parent_node_type,
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
        typechecker: &TypeChecker<F, C>,
        node: &CheckedAssignmentNode,
        path_node: &CheckedPathNode,
        symbols: &mut SymbolTable<F>,
        path: &mut Vec<usize>,
        parent_node_type: Option<NodeType>,
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
        parent_node_type: Option<NodeType>,
    ) -> Result<()> {
        let node = typechecker[stmt_id].as_variable().unwrap();
        let value = self
            .interpret_expr(typechecker, node.value, symbols, Some(node.node_type()))?
            .unwrap();

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
            _ => {
                unreachable!()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use qed_builder::{QExecContext, SymFeltEvalCache, SymFeltRef, SymFeltStore};
    use qed_fmt::Formatter;

    use super::*;

    #[test]
    fn test_interpreter() {
        qed_utils::setup_env_logger();

        insta::glob!("../../tests", "00*.qed", |path| {
            let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());
            let cache = SymFeltEvalCache::new();
            let store = SymFeltStore::new();
            let mut typecheker = TypeChecker::new();
            interpreter
                .interpret(&mut typecheker, path.to_path_buf(), vec![])
                .unwrap();
        });
    }
}
