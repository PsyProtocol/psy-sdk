use qed_ast::*;
use qed_builder::{Context, ContextFelt};
use tracing::instrument;

use crate::Interpreter;
impl<F: ContextFelt, C: Context<F>> AstVisitor<F> for Interpreter<F, C> {
    type ExprResult = ValueNode<F>;

    type StmtResult = ();

    #[instrument(skip_all)]
    fn visit_variable(&mut self, node: &VariableNode) -> Self::ExprResult {
        self.resolve_variable(&node.name)
    }

    #[instrument(skip_all)]
    fn visit_value(&mut self, node: &ValueNode<F>) -> Self::ExprResult {
        node.clone()
    }

    #[instrument(skip_all)]
    fn visit_binary(&mut self, node: &BinaryNode) -> Self::ExprResult {
        use BinaryOperator::*;
        let lhs = self
            .visit_expr(&*self.arena[node.lhs].clone())
            .try_as_felt()
            .unwrap();
        let rhs = self
            .visit_expr(&*self.arena[node.rhs].clone())
            .try_as_felt()
            .unwrap();
        ValueNode::Felt(match node.operator {
            Add => self.ctx.op_add(lhs, rhs),
            Sub => self.ctx.op_sub(lhs, rhs),
            Mul => self.ctx.op_mul(lhs, rhs),
            Div => self.ctx.op_div(lhs, rhs),
            Mod => self.ctx.op_mod(lhs, rhs),
            BitShr => self.ctx.op_bit_shr(lhs, rhs),
            BitShl => self.ctx.op_bit_shl(lhs, rhs),
            BitAnd => self.ctx.op_bit_and(lhs, rhs),
            BitOr => self.ctx.op_bit_or(lhs, rhs),
            BitXor => self.ctx.op_bit_xor(lhs, rhs),
            And => self.ctx.op_bool_and(lhs, rhs),
            Or => self.ctx.op_bool_or(lhs, rhs),
            Eq => self.ctx.op_eq(lhs, rhs),
            Neq => self.ctx.op_neq(lhs, rhs),
            Lt => self.ctx.op_lt(lhs, rhs),
            Lte => self.ctx.op_lte(lhs, rhs),
            Gt => self.ctx.op_gt(lhs, rhs),
            Gte => self.ctx.op_gte(lhs, rhs),
        })
    }

    #[instrument(skip_all)]
    fn visit_unary(&mut self, node: &UnaryNode) -> Self::ExprResult {
        let rhs = self
            .visit_expr(&*self.arena[node.rhs].clone())
            .try_as_felt()
            .unwrap();
        ValueNode::Felt(match node.operator {
            UnaryOperator::Neg => self.ctx.op_neg(rhs),
            UnaryOperator::Not => self.ctx.op_bool_not(rhs),
        })
    }

    #[instrument(skip_all)]
    fn visit_call(&mut self, node: &CallNode) -> Self::ExprResult {
        unimplemented!()
    }

    #[instrument(skip_all)]
    fn visit_if(&mut self, node: &IfNode) -> Self::StmtResult {
        self.enter_scope();
        let predicate = self
            .visit_expr(&*self.arena[node.if_branch.predicate].clone())
            .try_as_felt()
            .unwrap();
        self.ctx.start_if_block(predicate);
        self.visit_block(&node.if_branch.body);
        self.exit_scope();

        for condition in &node.elseif_branch {
            self.enter_scope();
            let predicate = self
                .visit_expr(&*self.arena[condition.predicate].clone())
                .try_as_felt()
                .unwrap();
            self.ctx.start_else_if_block(predicate);
            self.visit_block(&condition.body);
            self.exit_scope();
        }

        if let Some(else_branch) = &node.else_branch {
            self.enter_scope();
            self.ctx.start_else_block();
            self.visit_block(&else_branch);
            self.exit_scope();
        }

        self.ctx.end_if_block();
    }

    #[instrument(skip_all)]
    fn visit_while(&mut self, node: &WhileNode) -> Self::StmtResult {
        // self.enter_scope();
        // loop {
        //     let predicate = self
        //         .visit_expr(&*self.arena[node.predicate].clone())
        //         .try_as_felt()
        //         .unwrap();
        //     if predicate {
        //         self.ctx.start_if_block(predicate);
        //         self.visit_block(&node.body);
        //         self.ctx.end_if_block();
        //     } else {
        //         break;
        //     }
        // }
        // self.exit_scope();
    }

    #[instrument(skip_all)]
    fn visit_block(&mut self, node: &BlockNode) -> Self::StmtResult {
        self.enter_scope();
        for &stmt in &node.stmts {
            self.visit_stmt(&*self.arena[stmt].clone());
        }
        self.exit_scope();
    }

    #[instrument(skip_all)]
    fn visit_assignment(&mut self, node: &AssignmentNode) -> Self::StmtResult {
        let lhs = self.visit_variable(&node.variable).try_as_felt().unwrap();
        let rhs = self
            .visit_expr(&*self.arena[node.value].clone())
            .try_as_felt()
            .unwrap();
        use AssignmentOperator::*;
        let new_value = match node.operator {
            Eq => rhs,
            AddAssign => self.ctx.op_add(lhs, rhs),
            SubAssign => self.ctx.op_sub(lhs, rhs),
            MulAssign => self.ctx.op_mul(lhs, rhs),
            DivAssign => self.ctx.op_div(lhs, rhs),
            ModAssign => self.ctx.op_mod(lhs, rhs),
            BitAndAssign => self.ctx.op_bit_and(lhs, rhs),
            BitOrAssign => self.ctx.op_bit_or(lhs, rhs),
            BitXorAssign => self.ctx.op_bit_xor(lhs, rhs),
            BitShlAssign => self.ctx.op_bit_shl(lhs, rhs),
            BitShrAssign => self.ctx.op_bit_shr(lhs, rhs),
        };
        let new_value = self.ctx.cset(lhs, new_value);
        self.set_variable(node.variable.name, ValueNode::Felt(new_value))
    }

    #[instrument(skip_all)]
    fn visit_var_decl(&mut self, node: &VarDeclNode) -> Self::StmtResult {
        let value = self.visit_expr(&*self.arena[node.value].clone());
        self.define_variable(&node.name, value);
    }

    #[instrument(skip_all)]
    fn visit_return(&mut self, expr: &ReturnNode) -> Self::StmtResult {
        unimplemented!()
    }

    #[instrument(skip_all)]
    fn visit_function(&mut self, node: &FunctionNode) -> Self::StmtResult {
        unimplemented!()
    }

    #[instrument(skip_all)]
    fn visit_struct(&mut self, node: &StructNode) -> Self::StmtResult {
        unimplemented!()
    }

    #[instrument(skip_all)]
    fn visit_enum(&mut self, node: &EnumNode) -> Self::StmtResult {
        unimplemented!()
    }

    fn visit_impl(&mut self, node: &ImplNode) -> Self::StmtResult {
        unimplemented!()
    }
}
