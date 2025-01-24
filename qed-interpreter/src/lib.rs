#![feature(try_trait_v2)]

mod control;
mod error;
mod preprocess;

use either::Either;
use error::{Error, Result};
use indexmap::IndexMap;
use preprocess::{PreprocessorContext, StorageProcessor};
use qed_ast::*;
use qed_builder::{ContextFelt, ContextInput, DPNContext};
use qed_fmt::{Formatter, FormatterContext};
use qed_parser::Parser;
use qed_sema::Error as SemaError;
use qed_sema::*;
use std::{collections::HashMap, fmt::Display, ops::Index, path::PathBuf};

use tracing::{debug, error, info, instrument, span, Level};

use crate::control::ControlState;

#[derive(Debug)]
pub struct Interpreter<F: Clone + From<u32>, C> {
    pub inputs: Vec<u64>,
    pub context: C,
    pub contract_state_tree_height: u16,
    pub contract_id: F,
    pub user_id: F,
    _marker: std::marker::PhantomData<F>,
}

impl<F: ContextFelt + From<u32>, C: DPNContext<F>> ContextInput for Interpreter<F, C> {
    fn get_input(&self, index: u64) -> u64 {
        self.inputs[index as usize]
    }
}

impl<F: ContextFelt + From<u32> + Display + 'static, C: DPNContext<F>> Interpreter<F, C> {
    pub fn new(context: C, contract_state_tree_height: u16, contract_id: F, user_id: F) -> Self {
        Self {
            inputs: vec![],
            context,
            contract_state_tree_height,
            contract_id,
            user_id,
            _marker: std::marker::PhantomData,
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret(
        &mut self,
        typechecker: &mut TypeChecker<F, CheckedValueOrNode<F>, C>,
        entry: PathBuf,
        parameters: Vec<CheckedValue<F>>,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<Option<CheckedValue<F>>> {
        let mut program = Program::new();
        let mut parser = Parser::new(&mut program);
        parser
            .parse(&mut self.context, entry)
            .map_err(|err| Error::ParseError(err.to_string()))?;

        let mut storage_preprocessor: StorageProcessor = StorageProcessor::new();
        let mut preprocessor_context: PreprocessorContext<'_, F, C> =
            PreprocessorContext::new(&mut program);
        storage_preprocessor.visit_program(&mut preprocessor_context);

        let mut formatter_context: FormatterContext<F, C> = FormatterContext::new(&program);

        let mut formatter = Formatter::new();
        formatter.visit_program(&mut formatter_context);
        println!("formatted:\n{}", formatter.get_output());
        println!("ast:\n{:#?}", program);

        let mut artifact = Artifact::new(program);
        typechecker.typecheck_program(symbols, &mut artifact)?;
        let scope_id = symbols[ModuleId::root()].scope_id;
        let type_id = symbols[scope_id]
            .types
            .get(&IdentId::MAIN.into())
            .ok_or(Error::UndefinedMain)?;
        let main_function = &symbols[*type_id];
        match main_function {
            Type::Function(function) => {
                assert_eq!(
                    parameters.len(),
                    function.parameters.len(),
                    "expceted {} parameters for main function, got {}",
                    function.parameters.len(),
                    parameters.len()
                );
            }
            _ => panic!("IdentId::MAIN is not a function"),
        }
        return Ok(self.interpret_function(
            typechecker,
            &artifact,
            symbols[type_id.clone()].clone().as_ref(),
            parameters,
            symbols,
        )?);
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_function(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedFunctionNode,
        parameters: Vec<CheckedValue<F>>,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<Option<CheckedValue<F>>> {
        for (i, (parameter, _, _)) in node.parameters.iter().enumerate() {
            symbols.set_variable(
                Some(node.scope_id),
                parameter,
                CheckedValueOrNode::from(parameters[i].clone()),
            )?;
        }

        match self.interpret_block(typechecker, artifact, node.body.as_ref().unwrap(), symbols)? {
            ControlState::Return(value) => Ok(Some(value)),
            ControlState::Normal => Ok(None),
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_if(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedIfNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<()> {
        let predicate = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[node.if_branch.predicate],
                symbols,
            )?
            .unwrap()
            .try_as_bool()
            .unwrap();
        self.context.start_if_block(predicate);
        self.interpret_block(typechecker, artifact, &node.if_branch.body, symbols);

        for condition in &node.elseif_branch {
            let predicate = self
                .interpret_expr(
                    typechecker,
                    artifact,
                    &typechecker[condition.predicate],
                    symbols,
                )?
                .unwrap()
                .try_as_bool()
                .unwrap();
            self.context.start_else_if_block(predicate);
            self.interpret_block(typechecker, artifact, &condition.body, symbols);
        }

        if let Some(else_branch) = &node.else_branch {
            self.context.start_else_block();
            self.interpret_block(typechecker, artifact, &else_branch, symbols);
        }

        self.context.end_if_block();
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_while(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedWhileNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<()> {
        loop {
            let predicate = self
                .interpret_expr(typechecker, artifact, &typechecker[node.predicate], symbols)?
                .unwrap()
                .try_as_bool()
                .unwrap();
            if self.context.get_bool_value(predicate) {
                self.context.start_if_block(predicate);
                self.interpret_block(typechecker, artifact, &node.body, symbols);
                self.context.end_if_block();
            } else {
                break Ok(());
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_block(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedBlockNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<ControlState<CheckedValue<F>>> {
        for &stmt in &node.stmts {
            match self
                .interpret_statement(typechecker, artifact, &typechecker[stmt], symbols)
                .expect("interpret statement failed")
            {
                ControlState::Normal => continue,
                state => return Ok(state),
            }
        }
        Ok(ControlState::Normal)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_statement(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedStmtNode<F>,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<ControlState<CheckedValue<F>>> {
        match node {
            CheckedStmtNode::If(r#if) => self.interpret_if(typechecker, artifact, r#if, symbols)?,
            CheckedStmtNode::While(r#while) => {
                self.interpret_while(typechecker, artifact, r#while, symbols)?
            }
            CheckedStmtNode::Block(block) => {
                return self.interpret_block(typechecker, artifact, r#block, symbols);
            }
            CheckedStmtNode::Assignment(r#assignment) => {
                self.interpret_assignment(typechecker, artifact, r#assignment, symbols)?
            }
            CheckedStmtNode::Variable(variable) => {
                self.interpret_variable(typechecker, artifact, variable, symbols)?
            }
            CheckedStmtNode::Definition(definition) => {}
            CheckedStmtNode::Expression(expr) => {
                self.interpret_expr(typechecker, artifact, expr, symbols)?;
            }
            CheckedStmtNode::Return(return_node) => {
                if let Some((expr, _)) = &return_node.ret {
                    let value = self
                        .interpret_expr(typechecker, artifact, &typechecker[*expr], symbols)?
                        .unwrap();
                    return Ok(ControlState::Return(value));
                } else {
                    return Ok(ControlState::Normal);
                }
            }
            CheckedStmtNode::Storage(storage) => {
                let offset = self
                    .interpret_expr(typechecker, artifact, &typechecker[storage.offset], symbols)?
                    .unwrap();
                let value = self
                    .interpret_expr(typechecker, artifact, &typechecker[storage.value], symbols)?
                    .unwrap();
                self.context
                    .op_set_state_felt(offset.try_as_felt().unwrap(), value.try_as_felt().unwrap());
                return Ok(ControlState::Normal);
            }
        }
        Ok(ControlState::Normal)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_value(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        value_node: &CheckedValueOrNode<F>,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<CheckedValue<F>> {
        if let Either::Right(right) = value_node {
            return Ok(right.clone());
        }

        Ok(match value_node.as_ref().left().unwrap() {
            CheckedValueNode::Felt(value) => CheckedValue::Felt(*value),
            CheckedValueNode::Bool(value) => CheckedValue::Bool(*value),
            CheckedValueNode::Array(type_id, elements) => {
                let mut values = Vec::new();
                for element in elements {
                    values.push(
                        self.interpret_expr(
                            typechecker,
                            artifact,
                            &typechecker[*element],
                            symbols,
                        )?
                        .unwrap(),
                    );
                }
                CheckedValue::Array(*type_id, values)
            }
            CheckedValueNode::Struct(type_id, field_values) => {
                let mut values = IndexMap::new();
                for (field, expr) in field_values {
                    values.insert(
                        field.clone(),
                        self.interpret_expr(typechecker, artifact, &typechecker[*expr], symbols)?
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
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        unary_node: &CheckedUnaryNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<CheckedValue<F>> {
        let rhs_value = self
            .interpret_expr(typechecker, artifact, &typechecker[unary_node.rhs], symbols)?
            .unwrap();

        Ok(match unary_node.operator {
            UnaryOperator::Neg => {
                CheckedValue::Felt(self.context.op_neg(rhs_value.try_as_felt().unwrap()))
            }
            UnaryOperator::Not => {
                if unary_node.type_id == BOOL_TYPE {
                    CheckedValue::Bool(self.context.op_bool_not(rhs_value.try_as_bool().unwrap()))
                } else if unary_node.type_id == FELT_TYPE {
                    CheckedValue::Bool(self.context.op_bool_not(rhs_value.try_as_felt().unwrap()))
                } else {
                    todo!()
                }
            }
        })
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_binary(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        binary_node: &CheckedBinaryNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<CheckedValue<F>> {
        use BinaryOperator::*;
        let lhs_value = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[binary_node.lhs],
                symbols,
            )?
            .unwrap();
        let rhs_value = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[binary_node.rhs],
                symbols,
            )?
            .unwrap();

        let value = match binary_node.operator {
            Add => self.context.op_add(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            Sub => self.context.op_sub(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            Mul => self.context.op_mul(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            Div => self.context.op_div(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            Mod => self.context.op_mod(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            BitShr => self.context.op_u32_shr(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            BitShl => self.context.op_u32_shl(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            BitAnd => self.context.op_u32_and(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            BitOr => self.context.op_u32_or(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            BitXor => self.context.op_u32_xor(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            And => self.context.op_bool_and(
                lhs_value.try_as_bool().unwrap(),
                rhs_value.try_as_bool().unwrap(),
            ),
            Or => self.context.op_bool_or(
                lhs_value.try_as_bool().unwrap(),
                rhs_value.try_as_bool().unwrap(),
            ),
            Eq => {
                if lhs_value.is_felt() {
                    self.context.op_eq(
                        lhs_value.try_as_felt().unwrap(),
                        rhs_value.try_as_felt().unwrap(),
                    )
                } else {
                    self.context.op_eq(
                        lhs_value.try_as_bool().unwrap(),
                        rhs_value.try_as_bool().unwrap(),
                    )
                }
            }
            Neq => {
                if lhs_value.is_felt() {
                    self.context.op_neq(
                        lhs_value.try_as_felt().unwrap(),
                        rhs_value.try_as_felt().unwrap(),
                    )
                } else {
                    self.context.op_neq(
                        lhs_value.try_as_bool().unwrap(),
                        rhs_value.try_as_bool().unwrap(),
                    )
                }
            }
            Lt => self.context.op_lt(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            Lte => self.context.op_lte(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            Gt => self.context.op_gt(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            Gte => self.context.op_gte(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
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
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        old_value: CheckedValue<F>,
        operator: AssignmentOperator,
        value: CheckedValue<F>,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<CheckedValue<F>> {
        let new_value = match operator {
            AssignmentOperator::Eq => value,
            AssignmentOperator::AddAssign => CheckedValue::Felt(self.context.op_add(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::SubAssign => CheckedValue::Felt(self.context.op_sub(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::MulAssign => CheckedValue::Felt(self.context.op_mul(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::DivAssign => CheckedValue::Felt(self.context.op_div(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::ModAssign => CheckedValue::Felt(self.context.op_mod(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitAndAssign => CheckedValue::Felt(self.context.op_u32_and(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitOrAssign => CheckedValue::Felt(self.context.op_u32_or(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitXorAssign => CheckedValue::Felt(self.context.op_u32_xor(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitShlAssign => CheckedValue::Felt(self.context.op_u32_shl(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitShrAssign => CheckedValue::Felt(self.context.op_u32_shr(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
        };
        Ok(new_value)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_expr(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedExprNode<F>,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<Option<CheckedValue<F>>> {
        match node {
            CheckedExprNode::Path(CheckedPathNode {
                name,
                type_id,
                scope_id,
            }) => {
                if let Some(variable) = symbols.get_variable(Some(scope_id.clone()), &name) {
                    return Ok(Some(self.interpret_value(
                        typechecker,
                        artifact,
                        variable.clone().value.as_ref().unwrap(),
                        symbols,
                    )?));
                } else if let Some(type_id) =
                    symbols.get_type_id(Some(scope_id.clone()), name.clone())
                {
                    return Ok(Some(CheckedValue::Type(type_id)));
                } else {
                    return Err(SemaError::UnresolvedVariable.into());
                }
            }
            CheckedExprNode::Storage(storage_read) => {
                let offset = self
                    .interpret_expr(
                        typechecker,
                        artifact,
                        &typechecker[storage_read.offset],
                        symbols,
                    )?
                    .unwrap();
                let value = self.context.op_get_state_felt(
                    self.contract_state_tree_height,
                    self.contract_id,
                    self.user_id,
                    offset.try_as_felt().unwrap(),
                );
                return Ok(Some(CheckedValue::Felt(value)));
            }
            CheckedExprNode::Value(value_node) => Ok(Some(self.interpret_value(
                typechecker,
                artifact,
                &CheckedValueOrNode::from(value_node.clone()),
                symbols,
            )?)),
            CheckedExprNode::Binary(binary_node) => Ok(Some(self.interpret_binary(
                typechecker,
                artifact,
                binary_node,
                symbols,
            )?)),
            CheckedExprNode::Unary(unary_node) => Ok(Some(self.interpret_unary(
                typechecker,
                artifact,
                unary_node,
                symbols,
            )?)),
            CheckedExprNode::Call(call_node) => {
                let f = self
                    .interpret_expr(
                        typechecker,
                        artifact,
                        &typechecker[call_node.variable],
                        symbols,
                    )?
                    .unwrap();
                let type_id = f.type_id();
                let mut parameters = Vec::new();
                if let Some(receiver) = call_node.receiver {
                    let receiver = self.interpret_expr(
                        typechecker,
                        artifact,
                        &typechecker[receiver],
                        symbols,
                    )?;
                    parameters.push(receiver.unwrap());
                };
                for arg in &call_node.args {
                    parameters.push(
                        self.interpret_expr(
                            typechecker,
                            artifact,
                            &typechecker[arg.clone()],
                            symbols,
                        )?
                        .unwrap(),
                    );
                }
                return self.interpret_function(
                    typechecker,
                    artifact,
                    &symbols[type_id].clone().as_ref(),
                    parameters,
                    symbols,
                );
            }
            CheckedExprNode::Cast(cast_node) => {
                let value = self
                    .interpret_expr(
                        typechecker,
                        artifact,
                        &typechecker[cast_node.value],
                        symbols,
                    )?
                    .unwrap();
                if value.is_felt() && cast_node.target_type == BOOL_TYPE {
                    return Ok(Some(CheckedValue::Bool(value.try_as_felt().unwrap())));
                } else if value.is_bool() && cast_node.target_type == FELT_TYPE {
                    return Ok(Some(CheckedValue::Felt(value.try_as_bool().unwrap())));
                } else {
                    unimplemented!()
                }
            }
            CheckedExprNode::IndexAccess(index_access_node) => {
                let a = self
                    .interpret_expr(
                        typechecker,
                        artifact,
                        &typechecker[index_access_node.value],
                        symbols,
                    )?
                    .unwrap();

                return Ok(Some(a[index_access_node.index].clone()));
            }
            CheckedExprNode::MemberAccess(member_access_node) => {
                let s = self
                    .interpret_expr(
                        typechecker,
                        artifact,
                        &typechecker[member_access_node.value],
                        symbols,
                    )?
                    .unwrap();
                if let CheckedValue::Struct(type_id, field_values) = s {
                    if let Some(value) = field_values.get(&member_access_node.field) {
                        return Ok(Some(value.clone()));
                    } else {
                        if let Type::Function(f) = &symbols[member_access_node.type_id] {
                            if f.name == member_access_node.field {
                                return Ok(Some(CheckedValue::Type(member_access_node.type_id)));
                            } else {
                                return Err(Error::SemaError(qed_sema::Error::UnresolvedMember));
                            }
                        } else {
                            return Err(Error::SemaError(qed_sema::Error::UnresolvedMember));
                        }
                    }
                } else {
                    unreachable!()
                }
            }
            CheckedExprNode::Storage(checked_storage_node) => todo!(),
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_assignment(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedAssignmentNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<()> {
        let value = self
            .interpret_expr(typechecker, artifact, &typechecker[node.value], symbols)?
            .unwrap();

        match &typechecker[node.variable] {
            CheckedExprNode::Path(checked_path_node) => {
                let start_scope = Some(checked_path_node.scope_id.clone());
                let variable = symbols
                    .get_variable(start_scope, &checked_path_node.name)
                    .unwrap()
                    .value
                    .clone();
                let old_value = variable.and_then(|x| x.right()).unwrap();
                let new_value = self.interpret_assignment_value(
                    typechecker,
                    artifact,
                    old_value.clone(),
                    node.operator,
                    value,
                    symbols,
                )?;
                let new_value = self.context.cset(
                    old_value.try_as_felt().unwrap(),
                    new_value.try_as_felt().unwrap(),
                );
                symbols.set_variable(
                    start_scope,
                    &checked_path_node.name,
                    CheckedValueOrNode::from(CheckedValue::Felt(new_value)),
                )?;
            }
            CheckedExprNode::MemberAccess(member_access_node) => {
                let name = typechecker[member_access_node.value].name();
                let scope_id = typechecker[member_access_node.value].scope_id();
                let mut s = self
                    .interpret_expr(
                        typechecker,
                        artifact,
                        &typechecker[member_access_node.value],
                        symbols,
                    )?
                    .unwrap();
                let old_value = s.get_field(member_access_node.field).cloned().unwrap();
                let new_value = self.interpret_assignment_value(
                    typechecker,
                    artifact,
                    old_value,
                    node.operator,
                    value,
                    symbols,
                )?;
                s.set_field(member_access_node.field, new_value);

                symbols.set_variable(scope_id, &name, CheckedValueOrNode::from(s));
                return Ok(());
            }
            CheckedExprNode::IndexAccess(index_access_node) => {
                if let CheckedExprNode::MemberAccess(member_access_node) =
                    &typechecker[index_access_node.value]
                {
                    let name = typechecker[member_access_node.value].name();
                    let scope_id = typechecker[member_access_node.value].scope_id();
                    let mut s = self
                        .interpret_expr(
                            typechecker,
                            artifact,
                            &typechecker[member_access_node.value],
                            symbols,
                        )?
                        .unwrap();

                    let a = s.get_mut_field(member_access_node.field).unwrap();
                    let old_value = a[index_access_node.index].clone();
                    let new_value = self.interpret_assignment_value(
                        typechecker,
                        artifact,
                        old_value,
                        node.operator,
                        value,
                        symbols,
                    )?;
                    a[index_access_node.index] = new_value;

                    symbols.set_variable(scope_id, &name, CheckedValueOrNode::from(s));
                } else if let CheckedExprNode::Path(CheckedPathNode {
                    name,
                    type_id,
                    scope_id,
                }) = &typechecker[index_access_node.value]
                {
                    let mut a = self
                        .interpret_expr(
                            typechecker,
                            artifact,
                            &typechecker[index_access_node.value],
                            symbols,
                        )?
                        .unwrap();
                    let old_value = a[index_access_node.index].clone();
                    let new_value = self.interpret_assignment_value(
                        typechecker,
                        artifact,
                        old_value,
                        node.operator,
                        value,
                        symbols,
                    )?;

                    a[index_access_node.index] = new_value;

                    symbols.set_variable(Some(scope_id.clone()), name, CheckedValueOrNode::from(a));
                };
            }
            _ => unimplemented!(),
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_variable(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedVariableNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> Result<()> {
        let value = self
            .interpret_expr(typechecker, artifact, &typechecker[node.value], symbols)?
            .unwrap();
        // typechecker.print_scope_hierarchy(node.scope_id, 0, &symbols, artifact);

        symbols.set_variable(
            Some(node.scope_id),
            &node.name,
            CheckedValueOrNode::from(value),
        )?;
        Ok(())
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

        insta::glob!("../../tests", "002.qed", |path| {
            let mut interpreter = Interpreter::<SymFeltRef, _>::new(
                QExecContext::new(),
                0,
                SymFeltRef::from(0),
                SymFeltRef::from(0),
            );
            let cache = SymFeltEvalCache::new();
            let store = SymFeltStore::new();
            let mut symbols = SymbolTable::new();
            let mut typecheker = TypeChecker::new();
            interpreter
                .interpret(&mut typecheker, path.to_path_buf(), vec![], &mut symbols)
                .unwrap();
        });
    }
}
