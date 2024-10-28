use crate::*;

pub trait AstVisitor<F> {
    type ExprResult;
    type StmtResult;

    fn visit_expr(&mut self, expr: &ExprNode<F>) -> Self::ExprResult {
        match expr {
            ExprNode::Variable(ident) => self.visit_variable(ident),
            ExprNode::Value(literal) => self.visit_value(literal),
            ExprNode::Binary(binary) => self.visit_binary(binary),
            ExprNode::Unary(unary) => self.visit_unary(unary),
            ExprNode::Call(call) => self.visit_call(call),
        }
    }

    fn visit_definition(&mut self, module: &DefinitionNode) {
        match module {
            DefinitionNode::Function(function) => self.visit_function(function),
            DefinitionNode::Struct(r#struct) => self.visit_struct(r#struct),
            DefinitionNode::Enum(r#enum) => self.visit_enum(r#enum),
            DefinitionNode::Impl(r#impl) => self.visit_impl(r#impl),
        };
    }

    fn visit_stmt(&mut self, stmt: &StmtNode) -> Self::StmtResult {
        match stmt {
            StmtNode::If(if_node) => self.visit_if(if_node),
            StmtNode::While(while_node) => self.visit_while(while_node),
            StmtNode::Block(block) => self.visit_block(block),
            StmtNode::Assignment(assignment) => self.visit_assignment(assignment),
            StmtNode::VarDecl(variable) => self.visit_var_decl(variable),
            StmtNode::Return(expr) => self.visit_return(expr),
            StmtNode::StructDecl(r#struct) => self.visit_struct(r#struct),
            StmtNode::EnumDecl(r#enum) => self.visit_enum(r#enum),
            StmtNode::FunctionDecl(function) => self.visit_function(function),
            StmtNode::Impl(r#impl) => self.visit_impl(r#impl),
        }
    }

    fn visit_variable(&mut self, node: &VariableNode) -> Self::ExprResult;
    fn visit_value(&mut self, node: &ValueNode<F>) -> Self::ExprResult;
    fn visit_binary(&mut self, node: &BinaryNode) -> Self::ExprResult;
    fn visit_unary(&mut self, node: &UnaryNode) -> Self::ExprResult;
    fn visit_call(&mut self, node: &CallNode) -> Self::ExprResult;

    fn visit_if(&mut self, node: &IfNode) -> Self::StmtResult;
    fn visit_while(&mut self, node: &WhileNode) -> Self::StmtResult;
    fn visit_block(&mut self, node: &BlockNode) -> Self::StmtResult;
    fn visit_assignment(&mut self, node: &AssignmentNode) -> Self::StmtResult;
    fn visit_var_decl(&mut self, node: &VarDeclNode) -> Self::StmtResult;
    fn visit_return(&mut self, expr: &ReturnNode) -> Self::StmtResult;

    fn visit_impl(&mut self, node: &ImplNode) -> Self::StmtResult;
    fn visit_function(&mut self, node: &FunctionNode) -> Self::StmtResult;
    fn visit_struct(&mut self, node: &StructNode) -> Self::StmtResult;
    fn visit_enum(&mut self, node: &EnumNode) -> Self::StmtResult;
}
