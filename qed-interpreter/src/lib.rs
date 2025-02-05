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
        let mut default_visitor_context: DefaultVisitorContext<'_, F, C> =
            DefaultVisitorContext::new(&mut program);
        storage_preprocessor.visit_program(&mut default_visitor_context);

        let mut formatter = Formatter::new();
        formatter.visit_program(&mut default_visitor_context);
        println!("formatted:\n{}", formatter.get_output());
        println!("ast:\n{:#?}", program);

        let mut artifact = Artifact::new(program);
        typechecker.typecheck_program(symbols, &mut artifact)?;
        let scope_id = symbols[ModuleId::root()].scope_id;
        let type_id = symbols[scope_id]
            .types
            .get(&IdentId::MAIN.into())
            .ok_or(Error::UndefinedMain)?;

        let f: &CheckedFunctionNode = symbols[*type_id].as_ref();
        assert_eq!(
            parameters.len(),
            f.parameters.len(),
            "expceted {} parameters for main function, got {}",
            f.parameters.len(),
            parameters.len()
        );

        return Ok(self.interpret_function(
            typechecker,
            &artifact,
            &f.clone(),
            parameters,
            symbols,
            Some(NodeType::Module),
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
        parent_node_type: Option<NodeType>,
    ) -> Result<Option<CheckedValue<F>>> {
        symbols.push_frame();
        let res = self.__interpret_function__(typechecker, artifact, node, parameters, symbols);
        symbols.pop_frame();
        res
    }

    fn __interpret_function__(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedFunctionNode,
        parameters: Vec<CheckedValue<F>>,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
    ) -> std::result::Result<Option<CheckedValue<F>>, Error> {
        for (i, (parameter, _, _)) in node.parameters.iter().enumerate() {
            symbols.set_variable(
                Some(node.scope_id),
                parameter,
                CheckedValueOrNode::from(parameters[i].clone()),
            )?;
        }

        match self.interpret_block(
            typechecker,
            artifact,
            node.body.as_ref().unwrap(),
            symbols,
            Some(node.node_type()),
        )? {
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
        parent_node_type: Option<NodeType>,
    ) -> Result<()> {
        let predicate = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[node.if_branch.predicate],
                symbols,
                Some(node.node_type()),
            )?
            .unwrap()
            .try_as_bool()
            .unwrap();
        self.context.start_if_block(predicate);
        self.interpret_block(
            typechecker,
            artifact,
            &node.if_branch.body,
            symbols,
            Some(node.node_type()),
        );

        for condition in &node.elseif_branch {
            let predicate = self
                .interpret_expr(
                    typechecker,
                    artifact,
                    &typechecker[condition.predicate],
                    symbols,
                    Some(node.node_type()),
                )?
                .unwrap()
                .try_as_bool()
                .unwrap();
            self.context.start_else_if_block(predicate);
            self.interpret_block(
                typechecker,
                artifact,
                &condition.body,
                symbols,
                Some(node.node_type()),
            );
        }

        if let Some(else_branch) = &node.else_branch {
            self.context.start_else_block();
            self.interpret_block(
                typechecker,
                artifact,
                &else_branch,
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
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedWhileNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<()> {
        loop {
            let predicate = self
                .interpret_expr(
                    typechecker,
                    artifact,
                    &typechecker[node.predicate],
                    symbols,
                    Some(node.node_type()),
                )?
                .unwrap()
                .try_as_bool()
                .unwrap();
            if self.context.get_bool_value(predicate) {
                self.context.start_if_block(predicate);
                self.interpret_block(
                    typechecker,
                    artifact,
                    &node.body,
                    symbols,
                    Some(node.node_type()),
                );
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
        parent_node_type: Option<NodeType>,
    ) -> Result<ControlState<CheckedValue<F>>> {
        for &stmt in &node.stmts {
            match self.interpret_statement(
                typechecker,
                artifact,
                &typechecker[stmt],
                symbols,
                Some(node.node_type()),
            )? {
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
        node: &CheckedStmtNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<ControlState<CheckedValue<F>>> {
        match node {
            CheckedStmtNode::If(r#if) => {
                self.interpret_if(typechecker, artifact, r#if, symbols, parent_node_type)?
            }
            CheckedStmtNode::While(r#while) => {
                self.interpret_while(typechecker, artifact, r#while, symbols, parent_node_type)?
            }
            CheckedStmtNode::Block(block) => {
                return self.interpret_block(
                    typechecker,
                    artifact,
                    r#block,
                    symbols,
                    parent_node_type,
                );
            }
            CheckedStmtNode::Assignment(r#assignment) => self.interpret_assignment(
                typechecker,
                artifact,
                r#assignment,
                symbols,
                parent_node_type,
            )?,
            CheckedStmtNode::Variable(variable) => {
                self.interpret_variable(typechecker, artifact, variable, symbols, parent_node_type)?
            }
            CheckedStmtNode::Definition(definition) => {}
            CheckedStmtNode::Expression(expr) => {
                self.interpret_expr(
                    typechecker,
                    artifact,
                    &typechecker[*expr],
                    symbols,
                    parent_node_type,
                )?;
            }
            CheckedStmtNode::Return(return_node) => {
                return self.interpret_ret(
                    typechecker,
                    artifact,
                    return_node,
                    symbols,
                    parent_node_type,
                );
            }
            CheckedStmtNode::Storage(storage) => {
                self.interpret_storage_write(
                    typechecker,
                    artifact,
                    storage,
                    symbols,
                    parent_node_type,
                )?;
            }
        }
        Ok(ControlState::Normal)
    }

    fn interpret_storage_write(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        storage: &CheckedStorageWriteNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<ControlState<CheckedValue<F>>> {
        let offset = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[storage.offset],
                symbols,
                Some(storage.node_type()),
            )?
            .unwrap();
        let value = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[storage.value],
                symbols,
                Some(storage.node_type()),
            )?
            .unwrap();
        self.context
            .op_set_state_felt(offset.try_as_felt().unwrap(), value.try_as_felt().unwrap());
        return Ok(ControlState::Normal);
    }

    fn interpret_ret(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        return_node: &CheckedReturnNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<ControlState<CheckedValue<F>>> {
        Ok(if let Some((expr, _)) = &return_node.ret {
            let value = self
                .interpret_expr(
                    typechecker,
                    artifact,
                    &typechecker[*expr],
                    symbols,
                    Some(return_node.node_type()),
                )?
                .unwrap();
            return Ok(ControlState::Return(value));
        } else {
            return Ok(ControlState::Normal);
        })
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_value(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        value_node: &CheckedValueOrNode<F>,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        if let Either::Right(right) = value_node {
            return Ok(right.clone());
        }

        let node = value_node.as_ref().left().unwrap();
        Ok(match node {
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
                            Some(node.node_type()),
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
                        self.interpret_expr(
                            typechecker,
                            artifact,
                            &typechecker[*expr],
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
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        unary_node: &CheckedUnaryNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        let rhs_value = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[unary_node.rhs],
                symbols,
                Some(unary_node.node_type()),
            )?
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
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        use BinaryOperator::*;
        let lhs_value = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[binary_node.lhs],
                symbols,
                Some(binary_node.node_type()),
            )?
            .unwrap();
        let rhs_value = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[binary_node.rhs],
                symbols,
                Some(binary_node.node_type()),
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
        old_value: &CheckedValue<F>,
        operator: AssignmentOperator,
        value: CheckedValue<F>,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        let new_value = match operator {
            AssignmentOperator::Eq => value,
            AssignmentOperator::AddAssign => CheckedValue::Felt(self.context.op_add(
                old_value.clone().try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::SubAssign => CheckedValue::Felt(self.context.op_sub(
                old_value.clone().try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::MulAssign => CheckedValue::Felt(self.context.op_mul(
                old_value.clone().try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::DivAssign => CheckedValue::Felt(self.context.op_div(
                old_value.clone().try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::ModAssign => CheckedValue::Felt(self.context.op_mod(
                old_value.clone().try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitAndAssign => CheckedValue::Felt(self.context.op_u32_and(
                old_value.clone().try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitOrAssign => CheckedValue::Felt(self.context.op_u32_or(
                old_value.clone().try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitXorAssign => CheckedValue::Felt(self.context.op_u32_xor(
                old_value.clone().try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitShlAssign => CheckedValue::Felt(self.context.op_u32_shl(
                old_value.clone().try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
            AssignmentOperator::BitShrAssign => CheckedValue::Felt(self.context.op_u32_shr(
                old_value.clone().try_as_felt().unwrap(),
                value.try_as_felt().unwrap(),
            )),
        };
        Ok(self.cset_variable(old_value, &new_value))
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_expr(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedExprNode<F>,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<Option<CheckedValue<F>>> {
        match node {
            CheckedExprNode::Path(path) => Ok(Some(self.interpret_path(
                typechecker,
                artifact,
                path,
                symbols,
                parent_node_type,
            )?)),
            CheckedExprNode::Storage(storage_read) => Ok(Some(self.interpret_storage_read(
                typechecker,
                artifact,
                storage_read,
                symbols,
                parent_node_type,
            )?)),
            CheckedExprNode::Value(value_node) => Ok(Some(self.interpret_value(
                typechecker,
                artifact,
                &CheckedValueOrNode::from(value_node.clone()),
                symbols,
                parent_node_type,
            )?)),
            CheckedExprNode::Binary(binary_node) => Ok(Some(self.interpret_binary(
                typechecker,
                artifact,
                binary_node,
                symbols,
                parent_node_type,
            )?)),
            CheckedExprNode::Unary(unary_node) => Ok(Some(self.interpret_unary(
                typechecker,
                artifact,
                unary_node,
                symbols,
                parent_node_type,
            )?)),
            CheckedExprNode::Call(call_node) => Ok(self.interpret_call(
                typechecker,
                artifact,
                call_node,
                symbols,
                parent_node_type,
            )?),
            CheckedExprNode::Cast(cast_node) => Ok(Some(self.interpret_cast(
                typechecker,
                artifact,
                cast_node,
                symbols,
                parent_node_type,
            )?)),
            CheckedExprNode::IndexAccess(index_access_node) => {
                Ok(Some(self.interpret_index_access(
                    typechecker,
                    artifact,
                    index_access_node,
                    symbols,
                    parent_node_type,
                )?))
            }
            CheckedExprNode::MemberAccess(member_access_node) => {
                Ok(Some(self.interpret_member_access(
                    typechecker,
                    artifact,
                    member_access_node,
                    symbols,
                    parent_node_type,
                )?))
            }
        }
    }

    fn interpret_member_access(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        member_access_node: &CheckedMemberAccessNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        let s = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[member_access_node.value],
                symbols,
                Some(member_access_node.node_type()),
            )?
            .unwrap();
        let (type_id, field_values) = s.as_struct().unwrap();
        Ok(
            if let Some(value) = field_values.get(&member_access_node.field) {
                return Ok(value.clone());
            } else if symbols[member_access_node.type_id]
                .as_function()
                .map(|f| f.name == member_access_node.field)
                .unwrap_or(false)
            {
                return Ok(CheckedValue::Type(member_access_node.type_id));
            } else {
                return Err(Error::SemaError(qed_sema::Error::UnresolvedMember));
            },
        )
    }

    fn interpret_index_access(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        index_access_node: &CheckedIndexAccessNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        let a = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[index_access_node.value],
                symbols,
                Some(index_access_node.node_type()),
            )?
            .unwrap();
        return Ok(a[index_access_node.index].clone());
    }

    fn interpret_cast(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        cast_node: &CheckedCastNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        let value = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[cast_node.value],
                symbols,
                Some(cast_node.node_type()),
            )?
            .unwrap();
        if value.is_felt() && cast_node.target_type == BOOL_TYPE {
            return Ok(CheckedValue::Bool(value.try_as_felt().unwrap()));
        } else if value.is_bool() && cast_node.target_type == FELT_TYPE {
            return Ok(CheckedValue::Felt(value.try_as_bool().unwrap()));
        } else {
            unimplemented!()
        }
    }

    fn interpret_call(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        call_node: &CheckedCallNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<Option<CheckedValue<F>>> {
        let f = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[call_node.variable],
                symbols,
                Some(call_node.node_type()),
            )?
            .unwrap();
        let mut parameters = Vec::new();
        for arg in call_node.receiver.iter().chain(call_node.args.iter()) {
            parameters.push(
                self.interpret_expr(
                    typechecker,
                    artifact,
                    &typechecker[arg.clone()],
                    symbols,
                    Some(call_node.node_type()),
                )?
                .unwrap(),
            );
        }
        return self.interpret_function(
            typechecker,
            artifact,
            &symbols[f.type_id()].clone().as_ref(),
            parameters,
            symbols,
            Some(call_node.node_type()),
        );
    }

    fn interpret_path(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        path: &CheckedPathNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        if let Some(variable) = symbols.get_variable(Some(path.scope_id.clone()), &path.name) {
            return Ok(self.interpret_value(
                typechecker,
                artifact,
                variable.clone().value.as_ref().unwrap(),
                symbols,
                parent_node_type,
            )?);
        } else {
            return Ok(CheckedValue::Type(path.type_id.clone()));
        }
    }

    fn interpret_storage_read(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        storage_read: &CheckedStorageReadNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<CheckedValue<F>> {
        let offset = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[storage_read.offset],
                symbols,
                Some(storage_read.node_type()),
            )?
            .unwrap();
        let value = self.context.op_get_state_felt(
            self.contract_state_tree_height,
            self.contract_id,
            self.user_id,
            offset.try_as_felt().unwrap(),
        );
        return Ok(CheckedValue::Felt(value));
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_assignment(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedAssignmentNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<()> {
        let value = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[node.value],
                symbols,
                Some(node.node_type()),
            )?
            .unwrap();
        let mut path = vec![];

        let (old_value, name, variable) = match &typechecker[node.variable] {
            CheckedExprNode::Path(path_node) => self.interpret_path_assignment(
                typechecker,
                artifact,
                node,
                path_node,
                symbols,
                &mut path,
                parent_node_type,
            )?,
            CheckedExprNode::MemberAccess(member_access_node) => self.interpret_member_assignment(
                typechecker,
                artifact,
                node,
                member_access_node,
                symbols,
                &mut path,
                parent_node_type,
            )?,
            CheckedExprNode::IndexAccess(index_access_node) => self.interpret_index_assignment(
                typechecker,
                artifact,
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
            artifact,
            &old_value,
            node.operator,
            value,
            symbols,
            parent_node_type,
        )?;

        let mut variable_value = symbols
            .get_variable(Some(variable.scope_id), &name)
            .unwrap()
            .value
            .and_then(|x| x.right())
            .unwrap();
        variable_value.set_path(&path, new_value.clone());

        symbols.set_variable(
            Some(variable.scope_id),
            &name,
            CheckedValueOrNode::from(variable_value),
        )?;

        Ok(())
    }

    fn interpret_index_assignment(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedAssignmentNode,
        index_access_node: &CheckedIndexAccessNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        path: &mut Vec<usize>,
        parent_node_type: Option<NodeType>,
    ) -> Result<(
        CheckedValue<F>,
        IdentId,
        CheckedVariable<CheckedValueOrNode<F>>,
    )> {
        let (inner_value, inner_var_name, inner_var) = match &typechecker[index_access_node.value] {
            CheckedExprNode::Path(checked_path_node) => self.interpret_path_assignment(
                typechecker,
                artifact,
                node,
                checked_path_node,
                symbols,
                path,
                parent_node_type,
            )?,
            CheckedExprNode::IndexAccess(checked_index_access_node) => self
                .interpret_index_assignment(
                    typechecker,
                    artifact,
                    node,
                    checked_index_access_node,
                    symbols,
                    path,
                    parent_node_type,
                )?,
            CheckedExprNode::MemberAccess(checked_member_access_node) => self
                .interpret_member_assignment(
                    typechecker,
                    artifact,
                    node,
                    checked_member_access_node,
                    symbols,
                    path,
                    parent_node_type,
                )?,
            _ => unreachable!(),
        };

        path.push(index_access_node.index);

        Ok((
            inner_value.as_array().unwrap().1[index_access_node.index].clone(),
            inner_var_name,
            inner_var,
        ))
    }

    fn interpret_member_assignment(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedAssignmentNode,
        member_access_node: &CheckedMemberAccessNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        path: &mut Vec<usize>,
        parent_node_type: Option<NodeType>,
    ) -> Result<(
        CheckedValue<F>,
        IdentId,
        CheckedVariable<CheckedValueOrNode<F>>,
    )> {
        let (inner_value, inner_var_name, inner_var) = match &typechecker[member_access_node.value]
        {
            CheckedExprNode::Path(checked_path_node) => self.interpret_path_assignment(
                typechecker,
                artifact,
                node,
                checked_path_node,
                symbols,
                path,
                parent_node_type,
            )?,
            CheckedExprNode::IndexAccess(checked_index_access_node) => self
                .interpret_index_assignment(
                    typechecker,
                    artifact,
                    node,
                    checked_index_access_node,
                    symbols,
                    path,
                    parent_node_type,
                )?,
            CheckedExprNode::MemberAccess(checked_member_access_node) => self
                .interpret_member_assignment(
                    typechecker,
                    artifact,
                    node,
                    checked_member_access_node,
                    symbols,
                    path,
                    parent_node_type,
                )?,
            _ => unreachable!(),
        };

        path.push(member_access_node.field.into());

        Ok((
            inner_value
                .as_struct()
                .unwrap()
                .1
                .get(&member_access_node.field)
                .cloned()
                .unwrap(),
            inner_var_name,
            inner_var,
        ))
    }

    fn interpret_path_assignment(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedAssignmentNode,
        path_node: &CheckedPathNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        path: &mut Vec<usize>,
        parent_node_type: Option<NodeType>,
    ) -> Result<(
        CheckedValue<F>,
        IdentId,
        CheckedVariable<CheckedValueOrNode<F>>,
    )> {
        let start_scope = Some(path_node.scope_id);
        let variable = symbols.get_variable(start_scope, &path_node.name).unwrap();
        Ok((
            variable.value.clone().and_then(|x| x.right()).unwrap(),
            path_node.name,
            variable,
        ))
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_variable(
        &mut self,
        typechecker: &TypeChecker<F, CheckedValueOrNode<F>, C>,
        artifact: &Artifact<F, C>,
        node: &CheckedVariableNode,
        symbols: &mut SymbolTable<CheckedValueOrNode<F>>,
        parent_node_type: Option<NodeType>,
    ) -> Result<()> {
        let value = self
            .interpret_expr(
                typechecker,
                artifact,
                &typechecker[node.value],
                symbols,
                Some(node.node_type()),
            )?
            .unwrap();

        symbols.set_variable(
            Some(node.scope_id),
            &node.name,
            CheckedValueOrNode::from(value),
        )?;
        Ok(())
    }

    fn cset_variable(
        &mut self,
        old_value: &CheckedValue<F>,
        new_value: &CheckedValue<F>,
    ) -> CheckedValue<F> {
        match (old_value, new_value) {
            (CheckedValue::Felt(o), CheckedValue::Felt(n)) => {
                CheckedValue::Felt(self.context.cset(o.clone(), n.clone()))
            }
            (CheckedValue::Bool(o), CheckedValue::Bool(n)) => {
                CheckedValue::Bool(self.context.cset(o.clone(), n.clone()))
            }
            (CheckedValue::Array(lhs_type_id, o), CheckedValue::Array(rhs_type_id, n))
                if lhs_type_id == rhs_type_id =>
            {
                let mut result = Vec::new();
                for (old_value, new_value) in o.iter().zip(n.iter()) {
                    result.push(self.cset_variable(old_value, new_value));
                }
                CheckedValue::Array(lhs_type_id.clone(), result)
            }
            (CheckedValue::Struct(lhs_type_id, o), CheckedValue::Struct(rhs_type_id, n))
                if lhs_type_id == rhs_type_id =>
            {
                let mut result = IndexMap::new();
                for ((old_field_name, old_field_value), (new_field_name, new_field_value)) in
                    o.iter().zip(n.iter())
                {
                    assert_eq!(old_field_name, new_field_name);
                    result.insert(
                        old_field_name.clone(),
                        self.cset_variable(old_field_value, new_field_value),
                    );
                }
                CheckedValue::Struct(lhs_type_id.clone(), result)
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
