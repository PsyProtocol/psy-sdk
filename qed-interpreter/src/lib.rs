#![feature(try_trait_v2)]

mod control;
mod error;
mod preprocess;

use crate::control::ControlState;
use error::{Error, Result};
use indexmap::IndexMap;
pub use preprocess::StorageProcessor;
use qed_ast::*;
use qed_crypto::hash::utils::gen_dapen_contract_function_method_id;
use qed_fmt::Formatter;
use qed_parser::Parser;
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
use tracing::instrument;

#[derive(Clone, Debug)]
pub struct Interpreter<F: Clone + From<u32>, C> {
    pub context: C,
    _marker: std::marker::PhantomData<F>,
}

impl<F: ContextFelt + From<u32>, C: DPNContext<F> + 'static> Evaluator<F, C> for Interpreter<F, C> {
    fn evaluate_expr(
        &mut self,
        program: &CheckedProgram<F>,
        expr_id: ExprId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> CheckedValueRef<F> {
        self.interpret_expr(program, expr_id, ctx).unwrap()
    }

    fn to_constant_value(&mut self, value: CheckedValueRef<F>) -> u64 {
        self.context.get_constant_value(value.to_value())
    }

    fn from_constant_value(&mut self, value: u32) -> CheckedValueRef<F> {
        let value_f = self.context.op_const_u32(value);
        CheckedValueRef::from_u32(value_f)
    }
}

impl<F: ContextFelt + From<u32>, C: DPNContext<F> + 'static> Interpreter<F, C> {
    pub fn new(context: C) -> Self {
        Self {
            context,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn to_input(&mut self, ty: TypeId, symbols: &SymbolTable<F>) -> CheckedValue<F> {
        match symbols[ty].clone() {
            Type::Felt(_f) => CheckedValue::Felt(self.context.add_input()),
            Type::Bool(_b) => CheckedValue::Bool(self.context.add_bool_input()),
            Type::U32(_u) => CheckedValue::U32(self.context.add_u32_input()),
            Type::Array(a) => {
                let mut result = Vec::new();
                let inner_ty = symbols[a.inner_ty].clone();

                let size = if let Some(CheckedConstNode {
                    scope_id,
                    name,
                    value,
                    ..
                }) = symbols[a.size].as_const()
                {
                    let value = symbols.get_constant(value.clone());
                    self.context.get_constant_value(value.to_u32()) as usize
                } else {
                    unreachable!()
                };

                for _ in 0..size {
                    result.push(CheckedValueRef::new_rc(self.to_input(a.inner_ty, symbols)));
                }
                let type_id = symbols
                    .get_type_id(Some(ScopeId::primitive()), symbols[ty].key())
                    .unwrap();
                CheckedValue::Array(type_id, result)
            }
            Type::Tuple(elements) => {
                let mut result = Vec::new();
                for element_type in elements {
                    let element_ty = symbols[element_type].clone();
                    let value = CheckedValueRef::new_rc(self.to_input(element_type, symbols));

                    result.push((element_type, value));
                }
                let type_id = symbols
                    .get_type_id(Some(ScopeId::primitive()), symbols[ty].key())
                    .unwrap();

                CheckedValue::Tuple {
                    type_id,
                    elements: result,
                }
            }
            Type::Struct(s) => {
                let mut result = IndexMap::new();
                for (field_name, (field_type, _)) in &s.fields {
                    result.insert(
                        field_name.clone(),
                        CheckedValueRef::new_rc(self.to_input(field_type.clone(), symbols)),
                    );
                }
                let type_id = symbols
                    .get_type_id(Some(s.scope_id), symbols[ty].key())
                    .unwrap();
                CheckedValue::Struct(type_id, result)
            }
            _ => unreachable!(),
        }
    }

    pub fn size_of(&self, type_id: TypeId, symbols: &SymbolTable<F>) -> usize {
        match &symbols[type_id] {
            Type::Felt(_f) => 1usize,
            Type::Bool(_b) => 1usize,
            Type::U32(_u) => 1usize,
            // Type::Array(a) => self.size_of(a.inner_ty, ctx) * a.size,
            Type::Struct(s) => s
                .fields
                .iter()
                .map(|(_, (type_id, _))| self.size_of(type_id.clone(), symbols))
                .sum(),
            _ => unreachable!(),
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
        let (program, mut ctx) = self.typecheck(entry)?;

        let scope_id = ctx.symbols[ModuleId::root()].scope_id;
        let type_ids = if let Some(contract_name) = contract_name {
            let contract_name = ctx.program.interner.intern_ident(contract_name.into());
            let type_id = ctx.symbols[scope_id]
                .types
                .get(&contract_name.into())
                .ok_or(Error::UndefinedFunction)?
                .clone();

            method_names
                .into_iter()
                .map(|method_name| {
                    let method_name = ctx.program.interner.intern_ident(method_name.into());
                    ctx.symbols
                        .resolve_method(type_id, method_name)
                        .ok_or(Error::from(SemaError::UnresolvedMember))
                })
                .collect::<Result<Vec<TypeId>>>()?
        } else {
            method_names
                .into_iter()
                .map(|method_name| {
                    let method_name = ctx.program.interner.intern_ident(method_name.into());
                    ctx.symbols[scope_id]
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
            let node: &CheckedFunctionNode = ctx.symbols[type_id.clone()].as_ref();

            let mut parameters = vec![];
            for (_, _, parameter_type) in node.parameters.iter() {
                parameters.push(CheckedValueRef::new_rc(
                    self.to_input(parameter_type.clone(), &ctx.symbols),
                ));
            }
            let res = self.__interpret__(&program, type_id, parameters, &mut ctx)?;
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
        let (program, mut ctx) = self.typecheck(entry)?;

        let mut type_ids = Vec::new();

        let mut visited = HashMap::new();
        ctx.program
            .dependency_graph
            .clone()
            .ts(&ModuleId::root(), &mut visited, &mut |&module_id| {
                let scope_id = ctx.symbols[module_id].scope_id;
                let functions = ctx.symbols[scope_id]
                    .types
                    .iter()
                    .filter(|(_, &v)| {
                        ctx.symbols[v.clone()].is_function()
                            && ctx.symbols[v.clone()]
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
                assert!(ctx.symbols[type_id]
                    .as_function()
                    .unwrap()
                    .parameters
                    .is_empty());
                let res = self.__interpret__(&program, type_id, vec![], &mut ctx)?;
                outputs.push(compile_fn(&self.context, res));
                // resotre context
                self.context = context.clone();
            }
        }

        Ok(outputs)
    }

    fn __interpret__(
        &mut self,
        program: &CheckedProgram<F>,
        type_id: TypeId,
        parameters: Vec<CheckedValueRef<F>>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<(String, u32, Vec<F>)>
    where
        F: 'static,
    {
        let node: &CheckedFunctionNode = ctx.symbols[type_id.clone()].as_ref();
        let method_name = ctx.program[node.name].to_string();
        let mut method_args = Vec::with_capacity(node.parameters.len());

        for (parameter_name, _, parameter_type) in node.parameters.iter() {
            method_args.push((
                ctx.program[parameter_name.clone()].to_string(),
                self.size_of(parameter_type.clone(), &ctx.symbols),
            ));
        }

        let outputs = self
            .interpret_function(&program, type_id, parameters, ctx)?
            .unwrap();

        let method_id = gen_dapen_contract_function_method_id(method_name.clone(), &method_args);

        Ok((method_name, method_id, outputs.to_felts()))
    }

    pub fn typecheck(
        &mut self,
        entry: PathBuf,
    ) -> Result<(CheckedProgram<F>, TypeCheckerVisitorContext<F, C>)>
    where
        F: 'static,
    {
        let mut program = Program::new();
        let mut parser = Parser::new(&mut program);
        parser
            .parse(&mut self.context, entry)
            .map_err(|err| Error::ParseError(err.to_string()))?;

        let mut typechecker = TypeChecker::new(CheckedProgram::new(), Box::new(self.clone()));

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
        Ok((typechecker.program, typechecker_context))
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_function(
        &mut self,
        program: &CheckedProgram<F>,
        type_id: TypeId,
        parameters: Vec<CheckedValueRef<F>>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        let scope_id = ctx.symbols[type_id].scope_id();
        ctx.symbols.enter_function(scope_id);
        let res = self.__interpret_function__(program, type_id, parameters, ctx);
        ctx.symbols.exit_function(scope_id);
        res
    }

    fn __interpret_function__(
        &mut self,
        program: &CheckedProgram<F>,
        type_id: TypeId,
        args: Vec<CheckedValueRef<F>>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        // TODO: remove clone
        let parameters = ctx.symbols[type_id].parameters();

        assert_eq!(
            args.len(),
            parameters.len(),
            "expected {} parameters for main function, got {}",
            parameters.len(),
            args.len()
        );

        for (i, (parameter, _, _)) in parameters.iter().enumerate() {
            ctx.symbols.set_variable(
                ctx.symbols[type_id].scope_id(),
                parameter,
                args[i].clone(),
            )?;
        }

        self.interpret_statement(program, ctx.symbols[type_id].body(), ctx)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_while(
        &mut self,
        program: &CheckedProgram<F>,
        stmt_id: StmtId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let node = program[stmt_id].as_while().unwrap();
        loop {
            let predicate = self.interpret_expr(program, node.predicate, ctx)?.to_bool();

            if !self.is_constant(predicate) {
                return Err(Error::UncertainLoopCondition);
            }

            if self.context.get_constant_value(predicate) == 1 {
                self.context.start_if_block(predicate);
                self.interpret_statement(program, node.body, ctx)?;
                self.context.end_if_block();
            } else {
                break Ok(());
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_for(
        &mut self,
        program: &CheckedProgram<F>,
        stmt_id: StmtId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let node = program[stmt_id].as_for().unwrap();
        let start = self.interpret_expr(program, node.start, ctx)?;
        let end_f = self.interpret_expr(program, node.end, ctx)?.to_value();

        if !self.is_constant(start.to_value()) || !self.is_constant(end_f) {
            return Err(Error::UncertainLoopCondition);
        }

        ctx.symbols.enter_block(node.scope_id);
        ctx.symbols
            .set_variable(node.scope_id, &node.variable, start)?;

        loop {
            let value_f = ctx
                .symbols
                .get_variable(Some(node.scope_id), &node.variable)
                .unwrap()
                .value
                .unwrap()
                .to_u32();
            if value_f != end_f {
                self.interpret_statement(program, node.body, ctx)?;
                let one = self.context.op_const_u32(1);
                let value = CheckedValueRef::from_u32(self.context.op_u32_add(value_f, one));
                ctx.symbols
                    .set_variable(node.scope_id, &node.variable, value)?;
            } else {
                ctx.symbols.exit_block();
                break Ok(());
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_statement(
        &mut self,
        program: &CheckedProgram<F>,
        stmt_id: StmtId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        let node = &program[stmt_id];
        match node {
            CheckedStmtNode::For(r#_for) => self.interpret_for(program, stmt_id, ctx)?,
            CheckedStmtNode::Assignment(r#_assignment) => {
                self.interpret_assignment(program, stmt_id, ctx)?
            }
            CheckedStmtNode::Variable(_variable) => {
                self.interpret_variable(program, stmt_id, ctx)?
            }
            CheckedStmtNode::While(r#_while) => self.interpret_while(program, stmt_id, ctx)?,
            CheckedStmtNode::Definition(_definition) => {}
            CheckedStmtNode::Expression(expr_id) => match &program[expr_id.clone()].node_type() {
                NodeType::BlockExpr => {
                    return Ok(ControlState::Normal(
                        self.interpret_expr(program, *expr_id, ctx)?,
                    ));
                }
                _ => {
                    self.interpret_expr(program, *expr_id, ctx)?;
                }
            },
            CheckedStmtNode::Return(_return_node) => {
                return self.interpret_ret(program, stmt_id, ctx);
            }
            CheckedStmtNode::Intrinsic(intrinsic_node) => match intrinsic_node {
                CheckedIntrinsicStmtNode::Assert { left, message } => {
                    let lhs_value = self.interpret_expr(program, left.clone(), ctx)?;
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
                    let lhs_value = self.interpret_expr(program, left.clone(), ctx)?;
                    let rhs_value = self.interpret_expr(program, right.clone(), ctx)?;

                    self.context.assert_eq(
                        lhs_value.to_value(),
                        rhs_value.to_value(),
                        Box::leak(message.clone().unwrap_or_default().into_boxed_str()),
                    );
                }
            },
        }
        Ok(ControlState::Normal(CheckedValueRef::new_rc(
            CheckedValue::Type(VOID_TYPE),
        )))
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_ret(
        &mut self,
        program: &CheckedProgram<F>,
        stmt_id: StmtId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        let return_node = program[stmt_id].as_return().unwrap();
        if let Some(expr) = &return_node.ret {
            let value = self.interpret_expr(program, *expr, ctx)?;
            return Ok(ControlState::Return(value));
        }
        Ok(ControlState::Normal(CheckedValueRef::new_rc(
            CheckedValue::Type(VOID_TYPE),
        )))
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_value(
        &mut self,
        program: &CheckedProgram<F>,
        node: &CheckedValueNode<F>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValue<F>> {
        Ok(match node {
            CheckedValueNode::Felt(value) => CheckedValue::Felt(*value),
            CheckedValueNode::Bool(value) => CheckedValue::Bool(*value),
            CheckedValueNode::U32(value) => CheckedValue::U32(*value),
            CheckedValueNode::Array(type_id, elements) => {
                let mut values = Vec::new();
                for element in elements {
                    values.push(self.interpret_expr(program, *element, ctx)?);
                }
                CheckedValue::Array(*type_id, values)
            }
            CheckedValueNode::Struct(type_id, field_values) => {
                let mut values = IndexMap::new();
                for (field_name, field_value) in field_values {
                    values.insert(
                        field_name.clone(),
                        self.interpret_expr(program, *field_value, ctx)?,
                    );
                }
                CheckedValue::Struct(*type_id, values)
            }
            CheckedValueNode::Type(type_id) => CheckedValue::Type(*type_id),
            CheckedValueNode::Tuple(type_id, elements) => {
                let mut values = Vec::new();
                for (elem_type, expr_id) in elements {
                    let value = self.interpret_expr(program, *expr_id, ctx)?;

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
        program: &CheckedProgram<F>,
        unary_node: &CheckedUnaryNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValue<F>> {
        let rhs_value = self.interpret_expr(program, unary_node.rhs, ctx)?;

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
        program: &CheckedProgram<F>,
        binary_node: &CheckedBinaryNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValue<F>> {
        use BinaryOperator::*;
        let lhs_value = self.interpret_expr(program, binary_node.lhs, ctx)?;
        let rhs_value = self.interpret_expr(program, binary_node.rhs, ctx)?;

        let value = match (
            &*lhs_value.borrow(),
            &*rhs_value.borrow(),
            binary_node.operator,
        ) {
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Add) => self.context.op_add(*l, *r),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Sub) => self.context.op_sub(*l, *r),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Mul) => self.context.op_mul(*l, *r),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Div) => self.context.op_div(*l, *r),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Pow) => self.context.op_exp(*l, *r),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Mod) => self.context.op_mod(*l, *r),
            (CheckedValue::Felt(_), CheckedValue::Felt(_), BitShr) => unimplemented!(),
            (CheckedValue::Felt(_), CheckedValue::Felt(_), BitShl) => unimplemented!(),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), BitAnd) => {
                self.context.op_u32_and(*l, *r)
            }
            (CheckedValue::Felt(l), CheckedValue::Felt(r), BitOr) => self.context.op_u32_or(*l, *r),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), BitXor) => {
                self.context.op_u32_xor(*l, *r)
            }
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Eq) => self.context.op_eq(*l, *r),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Neq) => self.context.op_neq(*l, *r),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Lt) => self.context.op_lt(*l, *r),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Lte) => self.context.op_lte(*l, *r),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Gt) => self.context.op_gt(*l, *r),
            (CheckedValue::Felt(l), CheckedValue::Felt(r), Gte) => self.context.op_gte(*l, *r),

            (CheckedValue::U32(l), CheckedValue::U32(r), Add) => self.context.op_u32_add(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), Sub) => self.context.op_u32_sub(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), Mul) => self.context.op_u32_mul(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), Div) => self.context.op_u32_div(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), BitShr) => self.context.op_u32_shr(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), BitShl) => self.context.op_u32_shl(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), BitAnd) => self.context.op_u32_and(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), BitOr) => self.context.op_u32_or(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), BitXor) => self.context.op_u32_xor(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), Eq) => self.context.op_eq(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), Neq) => self.context.op_neq(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), Lt) => self.context.op_lt(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), Lte) => self.context.op_lte(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), Gt) => self.context.op_gt(*l, *r),
            (CheckedValue::U32(l), CheckedValue::U32(r), Gte) => self.context.op_gte(*l, *r),

            (CheckedValue::Bool(l), CheckedValue::Bool(r), And) => self.context.op_bool_and(*l, *r),
            (CheckedValue::Bool(l), CheckedValue::Bool(r), Or) => self.context.op_bool_or(*l, *r),
            (CheckedValue::Bool(l), CheckedValue::Bool(r), Eq) => self.context.op_eq(*l, *r),
            (CheckedValue::Bool(l), CheckedValue::Bool(r), Neq) => self.context.op_neq(*l, *r),
            (CheckedValue::Bool(_), CheckedValue::Bool(_), Lt) => unimplemented!(),
            (CheckedValue::Bool(_), CheckedValue::Bool(_), Lte) => unimplemented!(),
            (CheckedValue::Bool(_), CheckedValue::Bool(_), Gt) => unimplemented!(),
            (CheckedValue::Bool(_), CheckedValue::Bool(_), Gte) => unimplemented!(),

            _ => unreachable!(),
        };

        match binary_node.type_id {
            id if id == BOOL_TYPE => Ok(CheckedValue::Bool(value)),
            id if id == U32_TYPE => Ok(CheckedValue::U32(value)),
            _ => Ok(CheckedValue::Felt(value)),
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_assignment_value(
        &mut self,
        _program: &CheckedProgram<F>,
        old_value: &CheckedValueRef<F>,
        operator: AssignmentOperator,
        value: CheckedValueRef<F>,
        _ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValueRef<F>> {
        use AssignmentOperator::*;

        let new_value = match (&*old_value.borrow(), &*value.borrow(), operator) {
            (_, _, Eq) => value.clone(),

            (CheckedValue::Felt(l), CheckedValue::Felt(r), AddAssign) => {
                CheckedValueRef::from_felt(self.context.op_add(*l, *r))
            }
            (CheckedValue::Felt(l), CheckedValue::Felt(r), SubAssign) => {
                CheckedValueRef::from_felt(self.context.op_sub(*l, *r))
            }
            (CheckedValue::Felt(l), CheckedValue::Felt(r), MulAssign) => {
                CheckedValueRef::from_felt(self.context.op_mul(*l, *r))
            }
            (CheckedValue::Felt(l), CheckedValue::Felt(r), DivAssign) => {
                CheckedValueRef::from_felt(self.context.op_div(*l, *r))
            }
            (CheckedValue::Felt(l), CheckedValue::Felt(r), ModAssign) => {
                CheckedValueRef::from_felt(self.context.op_mod(*l, *r))
            }
            (CheckedValue::Felt(_), CheckedValue::Felt(_), BitAndAssign) => unimplemented!(),
            (CheckedValue::Felt(_), CheckedValue::Felt(_), BitOrAssign) => unimplemented!(),
            (CheckedValue::Felt(_), CheckedValue::Felt(_), BitXorAssign) => unimplemented!(),
            (CheckedValue::Felt(_), CheckedValue::Felt(_), BitShlAssign) => unimplemented!(),
            (CheckedValue::Felt(_), CheckedValue::Felt(_), BitShrAssign) => unimplemented!(),

            (CheckedValue::U32(l), CheckedValue::U32(r), AddAssign) => {
                CheckedValueRef::from_u32(self.context.op_u32_add(*l, *r))
            }
            (CheckedValue::U32(l), CheckedValue::U32(r), SubAssign) => {
                CheckedValueRef::from_u32(self.context.op_u32_sub(*l, *r))
            }
            (CheckedValue::U32(l), CheckedValue::U32(r), MulAssign) => {
                CheckedValueRef::from_u32(self.context.op_u32_mul(*l, *r))
            }
            (CheckedValue::U32(l), CheckedValue::U32(r), DivAssign) => {
                CheckedValueRef::from_u32(self.context.op_u32_div(*l, *r))
            }
            (CheckedValue::U32(l), CheckedValue::U32(r), ModAssign) => {
                CheckedValueRef::from_u32(self.context.op_mod(*l, *r))
            }
            (CheckedValue::U32(l), CheckedValue::U32(r), BitAndAssign) => {
                CheckedValueRef::from_u32(self.context.op_u32_and(*l, *r))
            }
            (CheckedValue::U32(l), CheckedValue::U32(r), BitOrAssign) => {
                CheckedValueRef::from_u32(self.context.op_u32_or(*l, *r))
            }
            (CheckedValue::U32(l), CheckedValue::U32(r), BitXorAssign) => {
                CheckedValueRef::from_u32(self.context.op_u32_xor(*l, *r))
            }
            (CheckedValue::U32(l), CheckedValue::U32(r), BitShlAssign) => {
                CheckedValueRef::from_u32(self.context.op_u32_shl(*l, *r))
            }
            (CheckedValue::U32(l), CheckedValue::U32(r), BitShrAssign) => {
                CheckedValueRef::from_u32(self.context.op_u32_shr(*l, *r))
            }

            _ => unreachable!(),
        };

        self.cset_variable(old_value, &new_value);
        Ok(new_value)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_expr(
        &mut self,
        program: &CheckedProgram<F>,
        expr_id: ExprId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValueRef<F>> {
        let node = &program[expr_id];
        match node {
            CheckedExprNode::Path(path) => Ok(self.interpret_path(program, path, ctx)?),
            CheckedExprNode::Intrinsic(ctx_node) => Ok({
                match ctx_node {
                    CheckedIntrinsicExprNode::GetUserId { .. } => {
                        CheckedValueRef::from_felt(self.context.get_user_id())
                    }
                    CheckedIntrinsicExprNode::GetContractId { .. } => {
                        CheckedValueRef::from_felt(self.context.get_contract_id())
                    }
                    CheckedIntrinsicExprNode::GetCheckpointId { .. } => {
                        CheckedValueRef::from_felt(self.context.get_checkpoint_id())
                    }
                    CheckedIntrinsicExprNode::GetLastNonce { .. } => {
                        CheckedValueRef::from_felt(self.context.get_last_nonce())
                    }
                    CheckedIntrinsicExprNode::GetUserPublicKeyHash { type_id, .. } => {
                        CheckedValueRef::from_vec(
                            type_id.clone(),
                            self.context.get_user_public_key_hash(),
                        )
                    }
                    CheckedIntrinsicExprNode::GetStateHashAt {
                        slot_index,
                        type_id,
                    } => {
                        let slot_index = self
                            .interpret_expr(program, slot_index.clone(), ctx)?
                            .to_felt();
                        CheckedValueRef::from_vec(
                            type_id.clone(),
                            self.context.get_state_hash_at(slot_index),
                        )
                    }
                    CheckedIntrinsicExprNode::GetOtherContractStateHashAt {
                        contract_state_tree_height,
                        contract_id,
                        slot_index,
                        type_id,
                    } => {
                        let contract_state_tree_height = self
                            .interpret_expr(program, contract_state_tree_height.clone(), ctx)?
                            .to_felt();
                        let contract_id = self
                            .interpret_expr(program, contract_id.clone(), ctx)?
                            .to_felt();
                        let slot_index = self
                            .interpret_expr(program, slot_index.clone(), ctx)?
                            .to_felt();
                        CheckedValueRef::from_vec(
                            type_id.clone(),
                            self.context.get_other_contract_state_hash_at(
                                contract_state_tree_height,
                                contract_id,
                                slot_index,
                            ),
                        )
                    }
                    CheckedIntrinsicExprNode::GetOtherUserContractStateHashAt {
                        contract_state_tree_height,
                        user_id,
                        contract_id,
                        slot_index,
                        type_id,
                    } => {
                        let contract_state_tree_height = self
                            .interpret_expr(program, contract_state_tree_height.clone(), ctx)?
                            .to_felt();
                        let user_id = self
                            .interpret_expr(program, user_id.clone(), ctx)?
                            .to_felt();
                        let contract_id = self
                            .interpret_expr(program, contract_id.clone(), ctx)?
                            .to_felt();
                        let slot_index = self
                            .interpret_expr(program, slot_index.clone(), ctx)?
                            .to_felt();
                        CheckedValueRef::from_vec(
                            type_id.clone(),
                            self.context.get_other_user_contract_state_hash_at(
                                contract_state_tree_height,
                                user_id,
                                contract_id,
                                slot_index,
                            ),
                        )
                    }
                    CheckedIntrinsicExprNode::CSetStateHashAt {
                        slot_index,
                        new_value,
                        type_id,
                    } => {
                        let new_value = self
                            .interpret_expr(program, new_value.clone(), ctx)?
                            .to_array();
                        let slot_index = self
                            .interpret_expr(program, slot_index.clone(), ctx)?
                            .to_felt();
                        CheckedValueRef::from_vec(
                            type_id.clone(),
                            self.context.cset_state_hash_at(slot_index, new_value),
                        )
                    }
                    CheckedIntrinsicExprNode::Read { offset, .. } => {
                        let contract_id = self.context.get_contract_id();
                        let user_id = self.context.get_user_id();

                        let offset = self.interpret_expr(program, offset.clone(), ctx)?;
                        let value = self.context.op_get_state_felt(
                            0,
                            contract_id,
                            user_id,
                            offset.to_felt(),
                        );
                        return Ok(CheckedValueRef::from_felt(value));
                    }
                    CheckedIntrinsicExprNode::Write { offset, value, .. } => {
                        let offset = self.interpret_expr(program, offset.clone(), ctx)?;
                        let value = self.interpret_expr(program, value.clone(), ctx)?;
                        return Ok(CheckedValueRef::from_felt(
                            self.context
                                .op_set_state_obj(offset.to_felt(), value.to_felt()),
                        ));
                    }
                    CheckedIntrinsicExprNode::Hash { data, type_id } => {
                        let data = self.interpret_expr(program, data.clone(), ctx)?;
                        return Ok(CheckedValueRef::from_vec(
                            type_id.clone(),
                            self.context.hash(&data.to_felts()),
                        ));
                    }
                }
            }),
            CheckedExprNode::Value(value_node) => Ok(CheckedValueRef::new_rc(
                self.interpret_value(program, &value_node, ctx)?,
            )),
            CheckedExprNode::Binary(binary_node) => Ok(CheckedValueRef::new_rc(
                self.interpret_binary(program, binary_node, ctx)?,
            )),
            CheckedExprNode::Unary(unary_node) => Ok(CheckedValueRef::new_rc(
                self.interpret_unary(program, unary_node, ctx)?,
            )),
            CheckedExprNode::Call(call_node) => {
                Ok(self.interpret_call(program, call_node, ctx)?.unwrap())
            }
            CheckedExprNode::MemberCall(checked_member_call_node) => Ok(self
                .interpret_member_call(program, checked_member_call_node, ctx)?
                .unwrap()),
            CheckedExprNode::Cast(cast_node) => Ok(CheckedValueRef::new_rc(
                self.interpret_cast(program, cast_node, ctx)?,
            )),
            CheckedExprNode::IndexAccess(index_access_node) => {
                Ok(self.interpret_index_access(program, index_access_node, ctx)?)
            }
            CheckedExprNode::TupleAccess(tuple_access_node) => {
                Ok(self.interpret_tuple_access(program, tuple_access_node, ctx)?)
            }
            CheckedExprNode::MemberAccess(member_access_node) => {
                Ok(self.interpret_member_access(program, member_access_node, ctx)?)
            }
            CheckedExprNode::LambdaFunction(checked_lambda_function_node) => Ok(
                CheckedValueRef::new_rc(CheckedValue::Type(checked_lambda_function_node.type_id)),
            ),
            CheckedExprNode::BlockExpr(block_expr) => {
                Ok(self.interpret_block_expr(program, block_expr, ctx)?)
            }
            CheckedExprNode::IfExpr(if_expr) => Ok(self.interpret_if_expr(program, if_expr, ctx)?),
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_member_access(
        &mut self,
        program: &CheckedProgram<F>,
        member_access_node: &CheckedMemberAccessNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValueRef<F>> {
        let s = self.interpret_expr(program, member_access_node.target, ctx)?;
        if ctx.symbols[member_access_node.type_id].is_function() {
            return Ok(CheckedValueRef::new_rc(CheckedValue::Type(
                member_access_node.type_id,
            )));
        } else {
            Ok(s.get_path(
                &mut self.context,
                &[IndexPath::Normal(member_access_node.field.into())],
            )
            .unwrap())
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_index_access(
        &mut self,
        program: &CheckedProgram<F>,
        index_access_node: &CheckedIndexAccessNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValueRef<F>> {
        let a = self.interpret_expr(program, index_access_node.target, ctx)?;
        let index = self.interpret_expr(program, index_access_node.index, ctx)?;

        return Ok(a
            .get_path(&mut self.context, &[IndexPath::Felt(index.to_felt())])
            .unwrap());
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_tuple_access(
        &mut self,
        program: &CheckedProgram<F>,
        tuple_access_node: &CheckedTupleAccessNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValueRef<F>> {
        let t = self.interpret_expr(program, tuple_access_node.target, ctx)?;

        Ok(t.get_path(
            &mut self.context,
            &[IndexPath::Normal(tuple_access_node.index.into())],
        )
        .unwrap())
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_cast(
        &mut self,
        program: &CheckedProgram<F>,
        cast_node: &CheckedCastNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValue<F>> {
        let value = self
            .interpret_expr(program, cast_node.value, ctx)?
            .to_value();

        if self.is_constant(value) {
            let const_value = self.context.get_constant_value(value);
            match cast_node.target_type {
                BOOL_TYPE => {
                    if const_value > 1 {
                        return Err(Error::SemaError(qed_sema::Error::InvalidCast));
                    }
                }
                FELT_TYPE => {}
                U32_TYPE => {
                    if const_value > 0xffffffffu64 {
                        return Err(Error::SemaError(qed_sema::Error::InvalidCast));
                    }
                }
                _ => unimplemented!(),
            }
        }

        match cast_node.target_type {
            BOOL_TYPE => Ok(CheckedValue::Bool(self.context.op_cast_bool(value))),
            FELT_TYPE => Ok(CheckedValue::Felt(self.context.op_cast_felt(value))),
            U32_TYPE => Ok(CheckedValue::U32(self.context.op_cast_u32(value))),
            _ => unimplemented!(),
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_call(
        &mut self,
        program: &CheckedProgram<F>,
        call_node: &CheckedCallNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        let f = self.interpret_expr(program, call_node.callee, ctx)?;
        let mut parameters = Vec::new();
        for arg in call_node.args.iter() {
            parameters.push(self.interpret_expr(program, arg.clone(), ctx)?);
        }
        return self.interpret_function(program, f.type_id(), parameters, ctx);
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_member_call(
        &mut self,
        program: &CheckedProgram<F>,
        call_node: &CheckedMemberCallNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<ControlState<CheckedValueRef<F>>> {
        let f = self.interpret_expr(program, call_node.callee, ctx)?;
        let mut parameters = Vec::new();
        for arg in once(&call_node.receiver).chain(call_node.args.iter()) {
            parameters.push(self.interpret_expr(program, arg.clone(), ctx)?);
        }
        return self.interpret_function(program, f.type_id(), parameters, ctx);
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_path(
        &mut self,
        _program: &CheckedProgram<F>,
        path: &CheckedPathNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValueRef<F>> {
        if let Some(variable) = ctx.symbols.get_variable(Some(path.scope_id), &path.name) {
            return Ok(variable.value.clone().unwrap());
        } else if let Some(CheckedConstNode {
            scope_id,
            name,
            value,
            ..
        }) = ctx.symbols[path.type_id].as_const()
        {
            let value = ctx.symbols.get_constant(value.clone());
            return Ok(value);
        } else {
            return Ok(CheckedValueRef::new_rc(CheckedValue::Type(
                path.type_id.clone(),
            )));
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_assignment(
        &mut self,
        program: &CheckedProgram<F>,
        stmt_id: StmtId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let node = program[stmt_id].as_assignment().unwrap();
        let value = self.interpret_expr(program, node.value, ctx)?;
        let mut path = vec![];

        let (old_value, name, variable) =
            self.interpret_assignment_target(program, node, &program[node.target], &mut path, ctx)?;

        let new_value =
            self.interpret_assignment_value(program, &old_value, node.operator, value, ctx)?;

        let mut variable_value = variable.value.unwrap();
        variable_value.set_path(&mut self.context, &path, new_value)?;

        ctx.symbols
            .set_variable(variable.scope_id, &name, variable_value)?;

        Ok(())
    }

    fn interpret_assignment_target(
        &mut self,
        program: &CheckedProgram<F>,
        node: &CheckedAssignmentNode,
        expr_node: &CheckedExprNode<F>,
        path: &mut Vec<IndexPath<F>>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<(CheckedValueRef<F>, IdentId, CheckedVariable<F>)> {
        match expr_node {
            CheckedExprNode::Path(path_node) => {
                self.interpret_path_assignment(program, node, path_node, path, ctx)
            }
            CheckedExprNode::MemberAccess(member_access_node) => {
                self.interpret_member_assignment(program, node, member_access_node, path, ctx)
            }
            CheckedExprNode::IndexAccess(index_access_node) => {
                self.interpret_index_assignment(program, node, index_access_node, path, ctx)
            }
            CheckedExprNode::TupleAccess(tuple_access_node) => {
                self.interpret_tuple_assignment(program, node, tuple_access_node, path, ctx)
            }
            _ => unreachable!(),
        }
    }

    fn interpret_index_assignment(
        &mut self,
        program: &CheckedProgram<F>,
        node: &CheckedAssignmentNode,
        index_access_node: &CheckedIndexAccessNode,
        path: &mut Vec<IndexPath<F>>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<(CheckedValueRef<F>, IdentId, CheckedVariable<F>)> {
        // Get the base value recursively
        let (inner_value, inner_var_name, inner_var) = self.interpret_assignment_target(
            program,
            node,
            &program[index_access_node.target],
            path,
            ctx,
        )?;

        // Create the index path
        let index = IndexPath::Felt(
            self.interpret_expr(program, index_access_node.index, ctx)?
                .to_felt(),
        );

        path.push(index.clone());

        Ok((
            inner_value.get_path(&mut self.context, &[index]).unwrap(),
            inner_var_name,
            inner_var,
        ))
    }

    fn interpret_tuple_assignment(
        &mut self,
        program: &CheckedProgram<F>,
        node: &CheckedAssignmentNode,
        tuple_access_node: &CheckedTupleAccessNode,
        path: &mut Vec<IndexPath<F>>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<(CheckedValueRef<F>, IdentId, CheckedVariable<F>)> {
        // Get the tuple value recursively
        let (tuple_value, tuple_var_name, tuple_var) = self.interpret_assignment_target(
            program,
            node,
            &program[tuple_access_node.target],
            path,
            ctx,
        )?;

        // Create the index path
        let index = IndexPath::Normal(tuple_access_node.index);
        path.push(index.clone());

        // Get the tuple element
        let element_value = tuple_value.get_path(&mut self.context, &[index]).unwrap();

        Ok((element_value, tuple_var_name, tuple_var))
    }

    fn interpret_member_assignment(
        &mut self,
        program: &CheckedProgram<F>,
        node: &CheckedAssignmentNode,
        member_access_node: &CheckedMemberAccessNode,
        path: &mut Vec<IndexPath<F>>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<(CheckedValueRef<F>, IdentId, CheckedVariable<F>)> {
        // Get the struct value recursively
        let (inner_value, inner_var_name, inner_var) = self.interpret_assignment_target(
            program,
            node,
            &program[member_access_node.target],
            path,
            ctx,
        )?;

        // Create the member path
        let index = IndexPath::Normal(member_access_node.field.into());
        path.push(index.clone());

        Ok((
            inner_value.get_path(&mut self.context, &[index]).unwrap(),
            inner_var_name,
            inner_var,
        ))
    }

    fn interpret_path_assignment(
        &mut self,
        _program: &CheckedProgram<F>,
        _node: &CheckedAssignmentNode,
        path_node: &CheckedPathNode,
        _path: &mut Vec<IndexPath<F>>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<(CheckedValueRef<F>, IdentId, CheckedVariable<F>)> {
        let start_scope = Some(path_node.scope_id);
        let variable = ctx
            .symbols
            .get_variable(start_scope, &path_node.name)
            .unwrap();
        Ok((variable.value.clone().unwrap(), path_node.name, variable))
    }

    #[instrument(level = "debug", skip_all)]
    fn interpret_variable(
        &mut self,
        program: &CheckedProgram<F>,
        stmt_id: StmtId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let node = program[stmt_id].as_variable().unwrap();
        let value = self.interpret_expr(program, node.value, ctx)?;

        ctx.symbols
            .set_variable(node.scope_id, &node.name, value.clone())?;
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
            (CheckedValue::U32(u), CheckedValue::U32(ref mut n)) => {
                *n = self.context.cset(u.clone(), n.clone());
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
        program: &CheckedProgram<F>,
        block_expr: &CheckedBlockExprNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValueRef<F>> {
        ctx.symbols.enter_block(block_expr.scope_id);

        for stmt_id in &block_expr.stmts {
            match self.interpret_statement(program, *stmt_id, ctx)? {
                ControlState::Return(value) => {
                    return Ok(value);
                }
                ControlState::Normal(_) => {}
            }
        }

        ctx.symbols.exit_block();
        Ok(CheckedValueRef::new_rc(CheckedValue::Type(VOID_TYPE)))
    }

    #[instrument(level = "debug", skip_all)]
    pub fn interpret_if_expr(
        &mut self,
        program: &CheckedProgram<F>,
        if_expr: &CheckedIfExprNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedValueRef<F>> {
        let node = if_expr;
        let mut result: CheckedValueRef<F>;

        //calculate the predicate
        let predicate = self
            .interpret_expr(program, node.if_branch.predicate, ctx)?
            .to_bool();

        //enter the if block
        self.context.start_if_block(predicate);

        //use if block return value to initialize the result
        result = self
            .interpret_statement(program, node.if_branch.body, ctx)?
            .unwrap();

        for condition in &node.elseif_branches {
            let predicate = self
                .interpret_expr(program, condition.predicate, ctx)?
                .to_bool();

            self.context.start_else_if_block(predicate);
            let elseif_result = self
                .interpret_statement(program, condition.body, ctx)?
                .unwrap();
            result = self.context.cset(result, elseif_result);
        }

        if let Some(else_branch) = &node.else_branch {
            self.context.start_else_block();

            let else_result = self
                .interpret_statement(program, else_branch.clone(), ctx)?
                .unwrap();
            result = self.context.cset(result, else_result);
        }

        self.context.end_if_block();

        Ok(result)
    }

    pub fn is_constant(&self, value: F) -> bool {
        let constant_types = [
            DPNOpType::Constant,
            DPNOpType::ConstantTrue,
            DPNOpType::ConstantFalse,
            DPNOpType::ConstantU32,
        ];

        constant_types.contains(&self.context.get_op_type(value))
    }
}

#[cfg(test)]
mod tests {
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
            #[allow(static_mut_refs)]
            unsafe {
                STD_PRIMITIVE_SCOPE_ID.take().unwrap()
            };
        });
    }
}
