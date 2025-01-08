pub mod error;

use error::{Error, Result};
use qed_ast::*;
use qed_builder::{Context, ContextFelt, ContextInput};
use qed_fmt::Formatter;
use qed_parser::Parser;
use qed_sema::Error as SemaError;
use qed_sema::*;
use std::{fmt::Display, ops::Index, path::PathBuf};

use tracing::{debug, error, info, instrument, span, Level};

#[derive(Debug)]
pub struct Interpreter<F: Clone, C> {
    inputs: Vec<u64>,
    context: C,
    symbols: SymbolTable<CheckedValueNode<F>>,
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
        parameters: Vec<CheckedValueNode<F>>,
    ) -> Result<Option<CheckedValueNode<F>>> {
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
        if let Some(type_id) = self.symbols[scope_id].types.get(&IdentId::MAIN.into()) {
            if let Type::Function(ref f) = self.symbols[type_id.clone()] {
                return Ok(self.interpret_function(
                    typechecker,
                    &artifact,
                    &f.clone(),
                    parameters,
                )?);
            }
        }
        Err(Error::UndefinedMain)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_function(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        node: &CheckedFunctionNode,
        parameters: Vec<CheckedValueNode<F>>,
    ) -> Result<Option<CheckedValueNode<F>>> {
        for (i, (parameter, _, _)) in node.parameters.iter().enumerate() {
            self.symbols
                .set_variable(Some(node.scope_id), parameter, parameters[i].clone())?;
        }

        let mut ret: Option<CheckedValueNode<F>> = None;
        for &stmt in &node.body.stmts {
            if let CheckedStmtNode::Return(CheckedReturnNode {
                ret: Some((expr, _)),
            }) = typechecker[stmt]
            {
                eprintln!("DEBUGPRINT[37]: lib.rs:85 (after ) = typechecker[stmt])");
                ret = Some(
                    self.interpret_expr(typechecker, artifact, &typechecker[expr])?
                        .unwrap(),
                );
            } else {
                self.interpret_statement(typechecker, artifact, &typechecker[stmt])?;
            }
        }

        Ok(ret)
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
    ) -> Result<()> {
        for &stmt in &node.stmts {
            self.interpret_statement(typechecker, artifact, &typechecker[stmt])?;
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_statement(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        node: &CheckedStmtNode<F>,
    ) -> Result<()> {
        match node {
            CheckedStmtNode::If(r#if) => self.interpret_if(typechecker, artifact, r#if)?,
            CheckedStmtNode::While(r#while) => {
                self.interpret_while(typechecker, artifact, r#while)?
            }
            CheckedStmtNode::Block(block) => {
                self.interpret_block(typechecker, artifact, r#block)?
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
            CheckedStmtNode::Return(return_node) => {}
        }
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_expr(
        &mut self,
        typechecker: &TypeChecker<F, C>,
        artifact: &ParsingArtifact<F, C>,
        node: &CheckedExprNode<F>,
    ) -> Result<Option<CheckedValueNode<F>>> {
        match node {
            CheckedExprNode::Path(CheckedPathNode {
                name,
                type_id,
                scope_id,
            }) => {
                if let Some(variable) = self.symbols.get_variable(Some(scope_id.clone()), &name) {
                    // TODO: avoid clone
                    return Ok(Some(variable.value.clone().unwrap()));
                } else if let Some(type_id) = self
                    .symbols
                    .get_type_id(Some(scope_id.clone()), name.clone())
                {
                    return Ok(Some(CheckedValueNode::Type(type_id)));
                } else {
                    return Err(SemaError::UnresolvedVariable.into());
                }
            }
            CheckedExprNode::Value(value_node) => Ok(Some(value_node.clone())),
            CheckedExprNode::Binary(binary_node) => {
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
                                lhs_value.try_as_felt().unwrap(),
                                rhs_value.try_as_felt().unwrap(),
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
                                lhs_value.try_as_felt().unwrap(),
                                rhs_value.try_as_felt().unwrap(),
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
                    Ok(Some(CheckedValueNode::Bool(value)))
                } else {
                    Ok(Some(CheckedValueNode::Felt(value)))
                }
            }
            CheckedExprNode::Unary(unary_node) => {
                let rhs_value = self
                    .interpret_expr(typechecker, artifact, &typechecker[unary_node.rhs])?
                    .unwrap();

                Ok(match unary_node.operator {
                    UnaryOperator::Neg => Some(CheckedValueNode::Felt(
                        self.context.op_neg(rhs_value.try_as_felt().unwrap()),
                    )),
                    UnaryOperator::Not => {
                        if unary_node.type_id == BOOL_TYPE {
                            Some(CheckedValueNode::Bool(
                                self.context.op_bool_not(rhs_value.try_as_bool().unwrap()),
                            ))
                        } else {
                            todo!()
                        }
                    }
                })
            }
            CheckedExprNode::Call(call_node) => {
                let expr = self
                    .interpret_expr(typechecker, artifact, &typechecker[call_node.variable])?
                    .unwrap();
                if let CheckedValueNode::Type(type_id) = expr {
                    let mut parameters = Vec::new();
                    if let Some(receiver) = call_node.receiver {
                        let receiver =
                            self.interpret_expr(typechecker, artifact, &typechecker[receiver])?;
                        parameters.push(receiver.unwrap());
                    };
                    for arg in &call_node.args {
                        parameters.push(
                            self.interpret_expr(typechecker, artifact, &typechecker[arg.clone()])?
                                .unwrap(),
                        );
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
            CheckedExprNode::IndexAccess(index_access_node) => {
                let expr = self
                    .interpret_expr(typechecker, artifact, &typechecker[index_access_node.value])?
                    .unwrap();

                if expr.is_array() {
                    if let CheckedValueNode::Array(_, _, elements) = expr {
                        return self.interpret_expr(
                            typechecker,
                            artifact,
                            &typechecker[elements[index_access_node.index]],
                        );
                    } else {
                        unreachable!()
                    }
                } else {
                    unreachable!()
                }
            }
            CheckedExprNode::MemberAccess(member_access_node) => {
                if let Type::Function(ref f) = &self.symbols[member_access_node.type_id] {
                    return Ok(Some(CheckedValueNode::Type(member_access_node.type_id)));
                } else {
                    let expr = self
                        .interpret_expr(
                            typechecker,
                            artifact,
                            &typechecker[member_access_node.value],
                        )?
                        .unwrap();
                    if expr.is_struct() {
                        if let CheckedValueNode::Struct(_, field_values) = expr {
                            if let Some(expr) = field_values.get(&member_access_node.field) {
                                return self.interpret_expr(
                                    typechecker,
                                    artifact,
                                    &typechecker[*expr],
                                );
                            } else {
                                unreachable!()
                            }
                        } else {
                            unreachable!()
                        }
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
                let old_value = self
                    .symbols
                    .get_variable(start_scope, &checked_path_node.name)
                    .unwrap()
                    .value
                    .clone();
                let new_value = match node.operator {
                    AssignmentOperator::Eq => value,
                    AssignmentOperator::AddAssign => {
                        if let Some(old_value) = old_value {
                            CheckedValueNode::Felt(self.context.op_add(
                                old_value.try_as_felt().unwrap(),
                                value.try_as_felt().unwrap(),
                            ))
                        } else {
                            value
                        }
                    }
                    AssignmentOperator::SubAssign => {
                        if let Some(old_value) = old_value {
                            CheckedValueNode::Felt(self.context.op_sub(
                                old_value.try_as_felt().unwrap(),
                                value.try_as_felt().unwrap(),
                            ))
                        } else {
                            value
                        }
                    }
                    AssignmentOperator::MulAssign => {
                        if let Some(old_value) = old_value {
                            CheckedValueNode::Felt(self.context.op_mul(
                                old_value.try_as_felt().unwrap(),
                                value.try_as_felt().unwrap(),
                            ))
                        } else {
                            value
                        }
                    }
                    AssignmentOperator::DivAssign => {
                        if let Some(old_value) = old_value {
                            CheckedValueNode::Felt(self.context.op_div(
                                old_value.try_as_felt().unwrap(),
                                value.try_as_felt().unwrap(),
                            ))
                        } else {
                            value
                        }
                    }
                    AssignmentOperator::ModAssign => {
                        if let Some(old_value) = old_value {
                            CheckedValueNode::Felt(self.context.op_mod(
                                old_value.try_as_felt().unwrap(),
                                value.try_as_felt().unwrap(),
                            ))
                        } else {
                            value
                        }
                    }
                    AssignmentOperator::BitAndAssign => {
                        if let Some(old_value) = old_value {
                            CheckedValueNode::Felt(self.context.op_bit_and(
                                old_value.try_as_felt().unwrap(),
                                value.try_as_felt().unwrap(),
                            ))
                        } else {
                            value
                        }
                    }
                    AssignmentOperator::BitOrAssign => {
                        if let Some(old_value) = old_value {
                            CheckedValueNode::Felt(self.context.op_bit_or(
                                old_value.try_as_felt().unwrap(),
                                value.try_as_felt().unwrap(),
                            ))
                        } else {
                            value
                        }
                    }
                    AssignmentOperator::BitXorAssign => {
                        if let Some(old_value) = old_value {
                            CheckedValueNode::Felt(self.context.op_bit_xor(
                                old_value.try_as_felt().unwrap(),
                                value.try_as_felt().unwrap(),
                            ))
                        } else {
                            value
                        }
                    }
                    AssignmentOperator::BitShlAssign => {
                        if let Some(old_value) = old_value {
                            CheckedValueNode::Felt(self.context.op_bit_shl(
                                old_value.try_as_felt().unwrap(),
                                value.try_as_felt().unwrap(),
                            ))
                        } else {
                            value
                        }
                    }
                    AssignmentOperator::BitShrAssign => {
                        if let Some(old_value) = old_value {
                            CheckedValueNode::Felt(self.context.op_bit_shr(
                                old_value.try_as_felt().unwrap(),
                                value.try_as_felt().unwrap(),
                            ))
                        } else {
                            value
                        }
                    }
                };
                self.symbols
                    .set_variable(start_scope, &checked_path_node.name, new_value)?;
            }
            CheckedExprNode::MemberAccess(member_access_node) => {
                let lhs = self
                    .interpret_expr(
                        typechecker,
                        artifact,
                        &typechecker[member_access_node.value],
                    )?
                    .unwrap();
                if let CheckedValueNode::Struct(type_id, mut field_values) = lhs {
                    if let Some(expr) = field_values.get_mut(&member_access_node.field) {
                        let old_value = self
                            .interpret_expr(typechecker, artifact, &typechecker[*expr])?
                            .unwrap();
                        let new_value = match node.operator {
                            AssignmentOperator::Eq => value,
                            AssignmentOperator::AddAssign => {
                                CheckedValueNode::Felt(self.context.op_add(
                                    old_value.try_as_felt().unwrap(),
                                    value.try_as_felt().unwrap(),
                                ))
                            }
                            AssignmentOperator::SubAssign => {
                                CheckedValueNode::Felt(self.context.op_sub(
                                    old_value.try_as_felt().unwrap(),
                                    value.try_as_felt().unwrap(),
                                ))
                            }
                            AssignmentOperator::MulAssign => {
                                CheckedValueNode::Felt(self.context.op_mul(
                                    old_value.try_as_felt().unwrap(),
                                    value.try_as_felt().unwrap(),
                                ))
                            }
                            AssignmentOperator::DivAssign => {
                                CheckedValueNode::Felt(self.context.op_div(
                                    old_value.try_as_felt().unwrap(),
                                    value.try_as_felt().unwrap(),
                                ))
                            }
                            AssignmentOperator::ModAssign => {
                                CheckedValueNode::Felt(self.context.op_mod(
                                    old_value.try_as_felt().unwrap(),
                                    value.try_as_felt().unwrap(),
                                ))
                            }
                            AssignmentOperator::BitAndAssign => {
                                CheckedValueNode::Felt(self.context.op_bit_and(
                                    old_value.try_as_felt().unwrap(),
                                    value.try_as_felt().unwrap(),
                                ))
                            }
                            AssignmentOperator::BitOrAssign => {
                                CheckedValueNode::Felt(self.context.op_bit_or(
                                    old_value.try_as_felt().unwrap(),
                                    value.try_as_felt().unwrap(),
                                ))
                            }
                            AssignmentOperator::BitXorAssign => {
                                CheckedValueNode::Felt(self.context.op_bit_xor(
                                    old_value.try_as_felt().unwrap(),
                                    value.try_as_felt().unwrap(),
                                ))
                            }
                            AssignmentOperator::BitShlAssign => {
                                CheckedValueNode::Felt(self.context.op_bit_shl(
                                    old_value.try_as_felt().unwrap(),
                                    value.try_as_felt().unwrap(),
                                ))
                            }
                            AssignmentOperator::BitShrAssign => {
                                CheckedValueNode::Felt(self.context.op_bit_shr(
                                    old_value.try_as_felt().unwrap(),
                                    value.try_as_felt().unwrap(),
                                ))
                            }
                        };

                        if let Type::Struct(s) = &self.symbols[type_id] {
                            self.symbols.set_variable(
                                Some(s.scope_id),
                                &member_access_node.field,
                                new_value,
                            );
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
            CheckedExprNode::IndexAccess(index_access_node) => {
                let lhs = self
                    .interpret_expr(typechecker, artifact, &typechecker[index_access_node.value])?
                    .unwrap();

                // todo!()
                // if let CheckedValueNode::Array(type_id, _, _) = lhs {
                //     if let Type::Array(_, _, scope_id) = &self.symbols[type_id] {
                //         self.symbols.set_variable(
                //             Some(*scope_id),
                //             &member_access_node.field,
                //             value,
                //         );
                //     } else {
                //         unreachable!()
                //     }
                // } else {
                //     unreachable!()
                // }
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

        self.symbols
            .set_variable(Some(node.scope_id), &node.name, value)?;
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
