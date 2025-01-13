#![feature(try_trait_v2)]

mod control;
mod error;

use either::Either;
use error::{Error, Result};
use qed_ast::*;
use qed_builder::{Context, ContextFelt, ContextInput};
use qed_fmt::Formatter;
use qed_parser::Parser;
use qed_sema::Error as SemaError;
use qed_sema::*;
use std::{collections::HashMap, fmt::Display, ops::Index, path::PathBuf};

use tracing::{debug, error, info, instrument, span, Level};

use crate::control::ControlState;

#[derive(Debug)]
pub struct Interpreter<F: Clone, C> {
    inputs: Vec<u64>,
    context: C,
    symbols: SymbolTable<CheckedValueOrNode<F>>,
}

impl<F: ContextFelt, C: Context<F>> ContextInput for Interpreter<F, C> {
    fn get_input(&self, index: u64) -> u64 {
        self.inputs[index as usize]
    }
}

impl<F: ContextFelt + Display, C: Context<F>> Interpreter<F, C> {
    pub fn new(context: C) -> Self {
        Self {
            inputs: vec![],
            context,
            symbols: SymbolTable::new(),
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret(
        &mut self,
        typechecker: &mut TypeChecker<F, C>,
        entry: PathBuf,
        parameters: Vec<CheckedValueOrNode<F>>,
    ) -> Result<Option<CheckedValue<F>>> {
        let mut parser = Parser::new();
        let program = parser
            .parse(&mut self.context, entry)
            .map_err(|err| Error::ParseError(err.to_string()))?;
        let mut formatter = Formatter::new(&parser);
        formatter.visit_program(&program);
        println!("formatted:\n{}", formatter.get_output());
        println!("ast:\n{:#?}", program);
        let mut artifact = ParsingArtifact::new(parser, program);
        typechecker.typecheck_program(&mut self.symbols, &mut artifact)?;
        let scope_id = self.symbols[ModuleId::root()].scope_id;
        let type_id = self.symbols[scope_id]
            .types
            .get(&IdentId::MAIN.into())
            .ok_or(Error::UndefinedMain)?;
        if let Type::Function(ref f) = self.symbols[type_id.clone()] {
            return Ok(self.interpret_function(typechecker, &artifact, &f.clone(), parameters)?);
        } else {
            unreachable!()
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_function(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        node: &CheckedFunctionNode,
        parameters: Vec<CheckedValueOrNode<F>>,
    ) -> Result<Option<CheckedValue<F>>> {
        for (i, (parameter, _, _)) in node.parameters.iter().enumerate() {
            self.symbols
                .set_variable(Some(node.scope_id), parameter, parameters[i].clone())?;
        }

        match self.interpret_block(typechecker, artifact, &node.body)? {
            ControlState::Return(value) => Ok(Some(value)),
            ControlState::Normal => Ok(None),
            ControlState::Break(_) | ControlState::Continue(_) => {
                unreachable!()
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_if(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        node: &CheckedIfNode,
    ) -> Result<()> {
        let predicate = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[node.if_branch.predicate],
            )?
            .unwrap()
            .try_as_bool()
            .unwrap();
        self.context.start_if_block(predicate);
        self.interpret_block(typechecker, artifact, &node.if_branch.body);

        for condition in &node.elseif_branch {
            let predicate = self
                .interpret_expr(typechecker, artifact, &typechecker[condition.predicate])?
                .unwrap()
                .try_as_bool()
                .unwrap();
            self.context.start_else_if_block(predicate);
            self.interpret_block(typechecker, artifact, &condition.body);
        }

        if let Some(else_branch) = &node.else_branch {
            self.context.start_else_block();
            self.interpret_block(typechecker, artifact, &else_branch);
        }

        self.context.end_if_block();
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_while(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        node: &CheckedWhileNode,
    ) -> Result<()> {
        loop {
            let predicate = self
                .interpret_expr(typechecker, artifact, &typechecker[node.predicate])?
                .unwrap()
                .try_as_bool()
                .unwrap();
            if self.context.get_bool_value(predicate) {
                self.context.start_if_block(predicate);
                self.interpret_block(typechecker, artifact, &node.body);
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
        artifact: &ParsingArtifact<F, C>,
        node: &CheckedBlockNode,
    ) -> Result<ControlState<CheckedValue<F>>> {
        for &stmt in &node.stmts {
            match self.interpret_statement(typechecker, artifact, &typechecker[stmt])? {
                ControlState::Normal => continue,
                state => return Ok(state),
            }
        }
        Ok(ControlState::Normal)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_statement(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        node: &CheckedStmtNode<F>,
    ) -> Result<ControlState<CheckedValue<F>>> {
        match node {
            CheckedStmtNode::If(r#if) => self.interpret_if(typechecker, artifact, r#if)?,
            CheckedStmtNode::While(r#while) => {
                self.interpret_while(typechecker, artifact, r#while)?
            }
            CheckedStmtNode::Block(block) => {
                return self.interpret_block(typechecker, artifact, r#block);
            }
            CheckedStmtNode::Assignment(r#assignment) => {
                self.interpret_assignment(typechecker, artifact, r#assignment)?
            }
            CheckedStmtNode::Variable(variable) => {
                self.interpret_variable(typechecker, artifact, variable)?
            }
            CheckedStmtNode::Definition(definition) => {}
            CheckedStmtNode::Expression(expr) => {
                self.interpret_expr(typechecker, artifact, expr)?;
            }
            CheckedStmtNode::Return(return_node) => {
                if let Some((expr, _)) = &return_node.ret {
                    let value = self
                        .interpret_expr(typechecker, artifact, &typechecker[*expr])?
                        .unwrap();
                    return Ok(ControlState::Return(value));
                } else {
                    return Ok(ControlState::Normal);
                }
            }
        }
        Ok(ControlState::Normal)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_value(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        value_node: &CheckedValueNode<F>,
    ) -> Result<CheckedValue<F>> {
        Ok(match value_node {
            CheckedValueNode::Felt(value) => CheckedValue::Felt(*value),
            CheckedValueNode::Bool(value) => CheckedValue::Bool(*value),
            CheckedValueNode::Array(type_id, _, elements) => {
                let mut values = Vec::new();
                for element in elements {
                    values.push(
                        self.interpret_expr(typechecker, artifact, &typechecker[*element])?
                            .unwrap(),
                    );
                }
                CheckedValue::Array(*type_id, values.len(), values)
            }
            CheckedValueNode::Struct(type_id, field_values) => {
                let mut values = HashMap::new();
                for (field, expr) in field_values {
                    values.insert(
                        field.clone(),
                        self.interpret_expr(typechecker, artifact, &typechecker[*expr])?
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
        artifact: &ParsingArtifact<F, C>,
        unary_node: &CheckedUnaryNode,
    ) -> Result<CheckedValue<F>> {
        let rhs_value = self
            .interpret_expr(typechecker, artifact, &typechecker[unary_node.rhs])?
            .unwrap();

        Ok(match unary_node.operator {
            UnaryOperator::Neg => {
                CheckedValue::Felt(self.context.op_neg(rhs_value.try_as_felt().unwrap()))
            }
            UnaryOperator::Not => {
                if unary_node.type_id == BOOL_TYPE {
                    CheckedValue::Bool(self.context.op_bool_not(rhs_value.try_as_bool().unwrap()))
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
        artifact: &ParsingArtifact<F, C>,
        binary_node: &CheckedBinaryNode,
    ) -> Result<CheckedValue<F>> {
        use BinaryOperator::*;
        let lhs_value = self
            .interpret_expr(typechecker, artifact, &typechecker[binary_node.lhs])?
            .unwrap();
        let rhs_value = self
            .interpret_expr(typechecker, artifact, &typechecker[binary_node.rhs])?
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
            BitShr => self.context.op_bit_shr(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            BitShl => self.context.op_bit_shl(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            BitAnd => self.context.op_bit_and(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            BitOr => self.context.op_bit_or(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            BitXor => self.context.op_bit_xor(
                lhs_value.try_as_felt().unwrap(),
                rhs_value.try_as_felt().unwrap(),
            ),
            And | Or => self.context.op_bool_and(
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
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        old_value: CheckedValue<F>,
        operator: AssignmentOperator,
        value: CheckedValue<F>,
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
            AssignmentOperator::BitAndAssign => CheckedValue::Felt(self.context.op_bit_and(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitOrAssign => CheckedValue::Felt(self.context.op_bit_or(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitXorAssign => CheckedValue::Felt(self.context.op_bit_xor(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitShlAssign => CheckedValue::Felt(self.context.op_bit_shl(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitShrAssign => CheckedValue::Felt(self.context.op_bit_shr(
                old_value.try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
        };
        Ok(new_value)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_expr(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        node: &CheckedExprNode<F>,
    ) -> Result<Option<CheckedValue<F>>> {
        match node {
            CheckedExprNode::Path(CheckedPathNode {
                name,
                type_id,
                scope_id,
            }) => {
                if let Some(variable) = self.symbols.get_variable(Some(scope_id.clone()), &name) {
                    let value_node = variable.value.clone().unwrap();
                    if let Either::Right(right) = value_node {
                        return Ok(Some(right));
                    } else if let Either::Left(left) = value_node {
                        return Ok(Some(self.interpret_value(typechecker, artifact, &left)?));
                    } else {
                        unreachable!()
                    }
                } else if let Some(type_id) = self
                    .symbols
                    .get_type_id(Some(scope_id.clone()), name.clone())
                {
                    return Ok(Some(CheckedValue::Type(type_id)));
                } else {
                    return Err(SemaError::UnresolvedVariable.into());
                }
            }
            CheckedExprNode::Value(value_node) => Ok(Some(self.interpret_value(
                typechecker,
                artifact,
                value_node,
            )?)),
            CheckedExprNode::Binary(binary_node) => Ok(Some(self.interpret_binary(
                typechecker,
                artifact,
                binary_node,
            )?)),
            CheckedExprNode::Unary(unary_node) => Ok(Some(self.interpret_unary(
                typechecker,
                artifact,
                unary_node,
            )?)),
            CheckedExprNode::Call(call_node) => {
                let f = self
                    .interpret_expr(typechecker, artifact, &typechecker[call_node.variable])?
                    .unwrap();
                if let CheckedValue::Type(type_id) = f {
                    let mut parameters = Vec::new();
                    if let Some(receiver) = call_node.receiver {
                        let receiver =
                            self.interpret_expr(typechecker, artifact, &typechecker[receiver])?;
                        parameters.push(CheckedValueOrNode::from(receiver.unwrap()));
                    };
                    for arg in &call_node.args {
                        parameters.push(CheckedValueOrNode::from(
                            self.interpret_expr(typechecker, artifact, &typechecker[arg.clone()])?
                                .unwrap(),
                        ));
                    }
                    if let Type::Function(f) = self.symbols[type_id].clone() {
                        return self.interpret_function(typechecker, artifact, &f, parameters);
                    } else {
                        unreachable!()
                    }
                } else {
                    unreachable!()
                }
            }
            CheckedExprNode::Cast(cast_node) => {
                let value = self
                    .interpret_expr(typechecker, artifact, &typechecker[cast_node.value])?
                    .unwrap();
                if value.is_felt() && cast_node.target_type == BOOL_TYPE {
                    if let CheckedValue::Felt(value) = value {
                        return Ok(Some(CheckedValue::Bool(value)));
                    } else {
                        unreachable!()
                    }
                } else if value.is_bool() && cast_node.target_type == FELT_TYPE {
                    if let CheckedValue::Bool(value) = value {
                        return Ok(Some(CheckedValue::Felt(value)));
                    } else {
                        unreachable!()
                    }
                } else {
                    unreachable!()
                }
            }
            CheckedExprNode::IndexAccess(index_access_node) => {
                let a = self
                    .interpret_expr(typechecker, artifact, &typechecker[index_access_node.value])?
                    .unwrap();

                if let CheckedValue::Array(_, _, elements) = a {
                    return Ok(Some(elements[index_access_node.index].clone()));
                } else {
                    unreachable!()
                }
            }
            CheckedExprNode::MemberAccess(member_access_node) => {
                if let Type::Function(ref f) = &self.symbols[member_access_node.type_id] {
                    return Ok(Some(CheckedValue::Type(member_access_node.type_id)));
                } else {
                    let s = self
                        .interpret_expr(
                            typechecker,
                            artifact,
                            &typechecker[member_access_node.value],
                        )?
                        .unwrap();
                    if let CheckedValue::Struct(_, field_values) = s {
                        return Ok(field_values.get(&member_access_node.field).cloned());
                    } else {
                        unreachable!()
                    }
                }
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_assignment(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        node: &CheckedAssignmentNode,
    ) -> Result<()> {
        let value = self
            .interpret_expr(typechecker, artifact, &typechecker[node.value])?
            .unwrap();

        match &typechecker[node.variable] {
            CheckedExprNode::Path(checked_path_node) => {
                let start_scope = Some(checked_path_node.scope_id.clone());
                let variable = self
                    .symbols
                    .get_variable(start_scope, &checked_path_node.name)
                    .unwrap()
                    .value
                    .clone();
                let old_value = variable.and_then(|x| x.right()).unwrap();
                let new_value = self.interpret_assignment_value(
                    typechecker,
                    artifact,
                    old_value,
                    node.operator,
                    value,
                )?;
                self.symbols.set_variable(
                    start_scope,
                    &checked_path_node.name,
                    CheckedValueOrNode::from(new_value),
                )?;
            }
            CheckedExprNode::MemberAccess(member_access_node) => {
                let (name, scope_id) =
                    if let CheckedExprNode::Path(CheckedPathNode { name, scope_id, .. }) =
                        &typechecker[member_access_node.value]
                    {
                        (name, scope_id)
                    } else {
                        unreachable!()
                    };
                let mut s = self
                    .interpret_expr(
                        typechecker,
                        artifact,
                        &typechecker[member_access_node.value],
                    )?
                    .unwrap();
                if let CheckedValue::Struct(type_id, field_values) = &mut s {
                    let old_value = field_values
                        .get(&member_access_node.field)
                        .cloned()
                        .unwrap();
                    field_values.insert(
                        member_access_node.field,
                        self.interpret_assignment_value(
                            typechecker,
                            artifact,
                            old_value,
                            node.operator,
                            value,
                        )?,
                    );
                } else {
                    unreachable!()
                }

                self.symbols.set_variable(
                    Some(scope_id.clone()),
                    name,
                    CheckedValueOrNode::from(s),
                );
                return Ok(());
            }
            CheckedExprNode::IndexAccess(index_access_node) => {
                if let CheckedExprNode::MemberAccess(member_access_node) =
                    &typechecker[index_access_node.value]
                {
                    let (name, scope_id) =
                        if let CheckedExprNode::Path(CheckedPathNode { name, scope_id, .. }) =
                            &typechecker[member_access_node.value]
                        {
                            (name, scope_id)
                        } else {
                            unreachable!()
                        };
                    let mut s = self
                        .interpret_expr(
                            typechecker,
                            artifact,
                            &typechecker[member_access_node.value],
                        )?
                        .unwrap();

                    if let CheckedValue::Struct(type_id, field_values) = &mut s {
                        if let Some(CheckedValue::Array(_, _, elements)) =
                            field_values.get_mut(&member_access_node.field)
                        {
                            elements[index_access_node.index] = self.interpret_assignment_value(
                                typechecker,
                                artifact,
                                elements[index_access_node.index].clone(),
                                node.operator,
                                value,
                            )?;
                        } else {
                            unreachable!()
                        }
                    } else {
                        unreachable!()
                    }

                    self.symbols.set_variable(
                        Some(scope_id.clone()),
                        name,
                        CheckedValueOrNode::from(s),
                    );
                } else if let CheckedExprNode::Path(CheckedPathNode {
                    name,
                    type_id,
                    scope_id,
                }) = &typechecker[index_access_node.value]
                {
                    let mut s = self
                        .interpret_expr(
                            typechecker,
                            artifact,
                            &typechecker[index_access_node.value],
                        )?
                        .unwrap();

                    if let CheckedValue::Array(_, _, elements) = &mut s {
                        elements[index_access_node.index] = self.interpret_assignment_value(
                            typechecker,
                            artifact,
                            elements[index_access_node.index].clone(),
                            node.operator,
                            value,
                        )?;
                    } else {
                        unreachable!()
                    }

                    self.symbols.set_variable(
                        Some(scope_id.clone()),
                        name,
                        CheckedValueOrNode::from(s),
                    );
                };
            }
            _ => unimplemented!(),
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_variable(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        node: &CheckedVariableNode,
    ) -> Result<()> {
        let value = self
            .interpret_expr(typechecker, artifact, &typechecker[node.value])?
            .unwrap();
        typechecker.print_scope_hierarchy(&self.symbols, artifact, node.scope_id, 0);

        self.symbols.set_variable(
            Some(node.scope_id),
            &node.name,
            CheckedValueOrNode::from(value),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use qed_builder::{ExecContext, SymFeltEvalCache, SymFeltRef, SymFeltStore};
    use qed_fmt::Formatter;

    use super::*;

    #[test]
    fn test_interpreter() {
        qed_utils::setup_env_logger();

        insta::glob!("../../tests", "002.qed", |path| {
            let mut interpreter = Interpreter::<SymFeltRef, _>::new(ExecContext::new());
            let cache = SymFeltEvalCache::new();
            let store = SymFeltStore::new();
            let mut typecheker = TypeChecker::new();
            interpreter
                .interpret(&mut typecheker, path.to_path_buf(), vec![])
                .unwrap();
        });
    }
}
