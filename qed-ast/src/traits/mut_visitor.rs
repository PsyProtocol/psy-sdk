use std::collections::HashMap;

use crate::*;

pub trait MutAstVisitor<F: Clone, C> {
    fn visit_expr(&mut self, expr: &mut ExprNode<F>) {
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

    fn visit_definition(&mut self, module: &mut DefinitionNode) {
        match module {
            DefinitionNode::Function(function) => self.visit_function(function),
            DefinitionNode::Struct(r#struct) => self.visit_struct(r#struct),
            DefinitionNode::Enum(r#enum) => self.visit_enum(r#enum),
            DefinitionNode::Impl(r#impl) => self.visit_impl(r#impl),
        }
    }

    fn visit_stmt(&mut self, stmt: &mut StmtNode<F>) {
        match stmt {
            StmtNode::If(if_node) => self.visit_if(if_node),
            StmtNode::While(while_node) => self.visit_while(while_node),
            StmtNode::Block(block) => self.visit_block(block),
            StmtNode::Assignment(assignment) => self.visit_assignment(assignment),
            StmtNode::Variable(variable) => self.visit_variable(variable),
            StmtNode::Return(expr) => self.visit_return(expr),
            StmtNode::Definition(definition) => self.visit_definition(definition),
            StmtNode::Expression(expr) => self.visit_expr(expr),
        }
    }

    fn visit_use(&mut self, u: &mut UsePath);

    fn visit_module(&mut self, module: &mut RawModule) {
        for u in &mut module.uses {
            self.visit_use(u);
        }
        for definition in &mut module.definitions {
            self.visit_definition(definition);
        }
    }

    fn visit_program(&mut self, program: &mut Program) {
        let mut visited = HashMap::new();
        program
            .dependency_graph
            .dfs(&program.root_file_id, &mut visited, &mut |module| {
                self.visit_module(&mut program.modules.get_mut(module).unwrap());
            });
    }

    fn visit_path(&mut self, node: &mut PathNode);
    fn visit_index_access(&mut self, node: &mut IndexAccessNode);
    fn visit_member_access(&mut self, node: &mut MemberAccessNode);
    fn visit_value(&mut self, node: &mut ValueNode<F>);
    fn visit_binary(&mut self, node: &mut BinaryNode);
    fn visit_unary(&mut self, node: &mut UnaryNode);
    fn visit_call(&mut self, node: &mut CallNode);

    fn visit_if(&mut self, node: &mut IfNode);
    fn visit_while(&mut self, node: &mut WhileNode);
    fn visit_block(&mut self, node: &mut BlockNode);
    fn visit_assignment(&mut self, node: &mut AssignmentNode);
    fn visit_variable(&mut self, node: &mut VariableNode);
    fn visit_return(&mut self, expr: &mut ReturnNode);

    fn visit_impl(&mut self, node: &mut ImplNode);
    fn visit_function(&mut self, node: &mut FunctionNode);
    fn visit_struct(&mut self, node: &mut StructNode);
    fn visit_enum(&mut self, node: &mut EnumNode);
}
