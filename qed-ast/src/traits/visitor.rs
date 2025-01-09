use std::collections::HashMap;

use crate::*;

pub trait AstVisitor<F: Clone, C> {
    type ExprResult;
    type StmtResult: Default;

    fn visit_expr(&mut self, expr: &ExprNode<F>) -> Self::ExprResult {
        match expr {
            ExprNode::Path(ident) => self.visit_path(ident),
            ExprNode::Value(literal) => self.visit_value(literal),
            ExprNode::Binary(binary) => self.visit_binary(binary),
            ExprNode::Unary(unary) => self.visit_unary(unary),
            ExprNode::Call(call) => self.visit_call(call),
            ExprNode::IndexAccess(index_access_node) => self.visit_index_access(index_access_node),
            ExprNode::MemberAccess(member_access_node) => {
                self.visit_member_access(member_access_node)
            }
        }
    }

    fn visit_definition(&mut self, module: &DefinitionNode) -> Self::StmtResult {
        match module {
            DefinitionNode::Function(function) => self.visit_function(function),
            DefinitionNode::Struct(r#struct) => self.visit_struct(r#struct),
            DefinitionNode::Enum(r#enum) => self.visit_enum(r#enum),
            DefinitionNode::Impl(r#impl) => self.visit_impl(r#impl),
        }
    }

    fn visit_stmt(&mut self, stmt: &StmtNode<F>) -> Self::StmtResult {
        match stmt {
            StmtNode::If(if_node) => self.visit_if(if_node),
            StmtNode::While(while_node) => self.visit_while(while_node),
            StmtNode::Block(block) => self.visit_block(block),
            StmtNode::Assignment(assignment) => self.visit_assignment(assignment),
            StmtNode::Variable(variable) => self.visit_variable(variable),
            StmtNode::Return(expr) => self.visit_return(expr),
            StmtNode::Definition(definition) => self.visit_definition(definition),
            StmtNode::Expression(expr) => {
                self.visit_expr(expr);
                Self::StmtResult::default()
            }
        }
    }

    fn visit_use(&mut self, u: &UsePath);

    fn visit_module(&mut self, module: &RawModule) {
        eprintln!("DEBUGPRINT[9]: visitor.rs:51 (after fn visit_module(&mut self, module: &RawM…)");
        for u in &module.uses {
            self.visit_use(u);
        }

        for definition in &module.definitions {
            self.visit_definition(definition);
        }
    }

    fn visit_program(&mut self, program: &Program) {
        let mut visited = HashMap::new();
        program
            .dependency_graph
            .ts(&program.root_file_id, &mut visited, &mut |file_id| {
                self.visit_module(&program.modules.get(file_id).unwrap());
            });
    }

    fn visit_path(&mut self, node: &PathNode) -> Self::ExprResult;
    fn visit_index_access(&mut self, node: &IndexAccessNode) -> Self::ExprResult;
    fn visit_member_access(&mut self, node: &MemberAccessNode) -> Self::ExprResult;

    fn visit_value(&mut self, node: &ValueNode<F>) -> Self::ExprResult;
    fn visit_binary(&mut self, node: &BinaryNode) -> Self::ExprResult;
    fn visit_unary(&mut self, node: &UnaryNode) -> Self::ExprResult;
    fn visit_call(&mut self, node: &CallNode) -> Self::ExprResult;

    fn visit_if(&mut self, node: &IfNode) -> Self::StmtResult;
    fn visit_while(&mut self, node: &WhileNode) -> Self::StmtResult;
    fn visit_block(&mut self, node: &BlockNode) -> Self::StmtResult;
    fn visit_assignment(&mut self, node: &AssignmentNode) -> Self::StmtResult;
    fn visit_variable(&mut self, node: &VariableNode) -> Self::StmtResult;
    fn visit_return(&mut self, expr: &ReturnNode) -> Self::StmtResult;

    fn visit_impl(&mut self, node: &ImplNode) -> Self::StmtResult;
    fn visit_function(&mut self, node: &FunctionNode) -> Self::StmtResult;
    fn visit_struct(&mut self, node: &StructNode) -> Self::StmtResult;
    fn visit_enum(&mut self, node: &EnumNode) -> Self::StmtResult;
}
