use qed_ast::{IdentId, Visibility, VisitorContext};
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{
    CheckedEnumNode, CheckedFunctionNode, CheckedStructNode, CheckedTraitNode, ScopeId, Type,
    TypeCheckerVisitorContext, TypeId, VarId,
};

macro_rules! writeln {
    ($fmt:expr, $($arg:tt)*) => {
        $fmt.writeln(format!($($arg)*))
    };
}
macro_rules! write {
    ($fmt:expr, $($arg:tt)*) => {
        $fmt.write(format!($($arg)*))
    };
}
pub trait AstVisualizer<F: Clone + From<u32>, C>: VisitorContext<F, C> {
    type DebugResult;

    fn debug_scope(&self, scope_id: ScopeId) -> Self::DebugResult;
    fn debug_type(&self, type_id: TypeId) -> Self::DebugResult;

    fn debug_variable(&self, ident_id: IdentId, var_id: VarId) -> Self::DebugResult;
}

struct IndentFormatter {
    output: String,
    indent: usize,
}

impl IndentFormatter {
    const INDENT: &'static str = "    ";

    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    pub fn indent(&mut self) {
        self.indent += 1;
    }

    pub fn dedent(&mut self) {
        self.indent -= 1;
    }

    pub fn write<T: AsRef<str>>(&mut self, s: T) {
        self.output.push_str(s.as_ref());
    }

    pub fn writeln<T: AsRef<str>>(&mut self, s: T) {
        self.write_indent();
        self.output.push_str(s.as_ref());
        self.output.push_str("\n");
    }

    pub fn write_indent(&mut self) {
        self.output.push_str(&Self::INDENT.repeat(self.indent));
    }

    pub fn finish(self) -> String {
        self.output
    }
}

struct TypeCheckerVisitorVisualizerInner<'a, F: Clone + From<u32> + ContextFelt, C> {
    pub context: &'a TypeCheckerVisitorContext<F, C>,
}

impl<'a, F: Clone + From<u32> + ContextFelt, C> TypeCheckerVisitorVisualizerInner<'a, F, C> {
    pub fn debug_scope(&self, scope_id: ScopeId, fmt: &mut IndentFormatter) {
        let scope = &self.context.symbols[scope_id];
        writeln!(fmt, "{:?} ({:?})", scope_id, scope.kind);
        fmt.indent();

        if !scope.consts.is_empty() {
            writeln!(fmt, "Constants:");
            fmt.indent();
            for (indent_id, const_id) in &scope.consts {
                writeln!(fmt, "{:?}: {:?}", const_id, self.context.ident(*indent_id));
            }
            fmt.dedent();
        }

        if !scope.variables.is_empty() {
            writeln!(fmt, "Variables:");
            fmt.indent();
            for (indent_id, var_id) in &scope.variables {
                self.debug_variable_inline(*indent_id, *var_id, fmt);
                // writeln!(fmt, "{:?}: {:?}", var_id, self.context.ident(*indent_id));
            }
            fmt.dedent();
        }

        if !scope.types.is_empty() {
            writeln!(fmt, "Types:");
            fmt.indent();
            for (_type_key, type_id) in &scope.types {
                let ty = &self.context.symbols[*type_id];
                match &ty {
                    Type::Unknown | Type::VOID | Type::Bool(_) | Type::U32(_) => {}
                    _ => {
                        self.debug_type(*type_id, fmt);
                    }
                }
            }
            fmt.dedent();
        }

        for child in &scope.children {
            self.debug_scope(*child, fmt);
        }
        fmt.dedent();
    }

    pub fn debug_type_declaration(&self, type_id: TypeId, fmt: &mut IndentFormatter) {
        let ty = &self.context.symbols[type_id];
        let visibility = ty.visibility();
        fmt.write_indent();
        if visibility == Visibility::Public {
            write!(fmt, "pub ");
        };
        match &ty {
            Type::Struct(CheckedStructNode { name, .. }) => {
                let type_name = &self.context.ident(*name).0;
                write!(fmt, "struct {} ", type_name);
            }
            Type::Enum(CheckedEnumNode { name, .. }) => {
                let type_name = &self.context.ident(*name).0;
                write!(fmt, "enum {} ", type_name);
            }
            Type::Trait(CheckedTraitNode { name, .. }) => {
                let type_name = &self.context.ident(*name).0;
                write!(fmt, "trait {} ", type_name);
            }
            Type::Function(CheckedFunctionNode {
                name, qualifier, ..
            }) => {
                if qualifier.is_extern {
                    write!(fmt, "extern ");
                }
                if qualifier.is_const {
                    write!(fmt, "const ");
                }
                let type_name = &self.context.ident(*name).0;
                write!(fmt, "fn {} ", type_name);
            }
            Type::Const(node) => {
                write!(fmt, "{:?} ", self.context.symbols.get_constant(node.value));
            }
            Type::Array(_) => {
                write!(fmt, "Array ");
            }
            Type::GenericInstance(type_id, type_ids, _scope) => {
                match &self.context.symbols[*type_id] {
                    Type::Array(_) => {
                        let t = self.get_type_name(type_ids[0]);
                        let n = self.get_type_name(type_ids[1]);
                        write!(fmt, "[{}; {}] ", t, n);
                    }
                    Type::Struct(_) => {
                        self.debug_type(*type_id, fmt);
                    }
                    _ => {
                        unimplemented!()
                    }
                }
            }
            _ => {
                write!(fmt, "{:?} ", ty.kind());
            }
        };
        write!(fmt, "({:?})", type_id);
        write!(fmt, "\n");
    }

    pub fn debug_type(&self, type_id: TypeId, fmt: &mut IndentFormatter) {
        self.debug_type_declaration(type_id, fmt);
        match &self.context.symbols[type_id] {
            Type::Array(_) | Type::GenericInstance(_, _, _) | Type::Const(_) => return,
            _ => {}
        }
        fmt.indent();
        match &self.context.symbols[type_id] {
            Type::Unknown | Type::VOID => {}
            Type::U32(node) => {
                self.fmt_type_names("Implementations", &node.implementations, fmt);
            }
            Type::Felt(node) => {
                self.fmt_type_names("Implementations", &node.implementations, fmt);
            }
            Type::Bool(node) => {
                self.fmt_type_names("Implementations", &node.implementations, fmt);
            }
            Type::Struct(node) => {
                if !node.fields.is_empty() {
                    writeln!(fmt, "Fields:");
                    fmt.indent();
                    for (ident_id, (type_id, visibility)) in node.fields.iter() {
                        let ident_name = &self.context.ident(*ident_id).0;
                        fmt.write_indent();
                        if visibility == &Visibility::Public {
                            write!(fmt, "pub ");
                        };
                        let type_name = self.get_type_name(*type_id);
                        write!(
                            fmt,
                            "{} {} ({:?}, {:?})",
                            type_name, ident_name, ident_id, type_id
                        );
                        write!(fmt, "\n");
                    }
                    fmt.dedent();
                };
                if !node.generic_parameters.is_empty() {
                    writeln!(fmt, "Generic Parameters: {:?}", node.generic_parameters);
                }
                self.fmt_type_names("Implementations", &node.implementations, fmt);
            }
            Type::Function(node) => {
                if !node.parameters.is_empty() {
                    writeln!(fmt, "Parameters:");
                    fmt.indent();
                    for (ident_id, type_qualifier, type_id) in node.parameters.iter() {
                        fmt.write_indent();
                        if type_qualifier.is_mutable {
                            write!(fmt, "mut ")
                        }
                        let type_name = self.get_type_name(*type_id);
                        let ident_name = &self.context.ident(*ident_id).0;
                        write!(
                            fmt,
                            "{} {} ({:?}, {:?})",
                            type_name, ident_name, ident_id, type_id
                        );
                        write!(fmt, "\n");
                    }
                    fmt.dedent();
                };
                writeln!(
                    fmt,
                    "Return Type: {} ({:?})",
                    self.get_type_name(node.return_type),
                    node.return_type
                );
                if let Some(body) = node.body {
                    writeln!(fmt, "Body: {:?}", body);
                }
                if !node.generic_parameters.is_empty() {
                    writeln!(fmt, "Generic Parameters: {:?}", node.generic_parameters);
                }
                if !node.attrs.is_empty() {
                    writeln!(fmt, "Attrs: {:?}", node.attrs);
                }
            }
            Type::Trait(node) => {
                if !node.body.is_empty() {
                    writeln!(fmt, "Body: {:?}", node.body);
                }
                if !node.unchecked_body.is_empty() {
                    writeln!(fmt, "Unchecked Body: {:?}", node.unchecked_body);
                }
                if !node.generic_parameters.is_empty() {
                    writeln!(fmt, "Generic Parameters: {:?}", node.generic_parameters);
                }
                self.fmt_type_names("Implementor", &node.implementors, fmt);
            }
            Type::Array(node) => {
                writeln!(
                    fmt,
                    "[{}; {}]",
                    self.get_type_name(node.inner_ty),
                    self.get_type_name(node.size_ty),
                );
                self.fmt_type_names("Implementations", &node.implementations, fmt);
            }
            Type::TypeVariable(_) => {}
            Type::GenericInstance(_type_id, _type_ids, _scope_id) => {}
            x => writeln!(fmt, "Remaining {:?}", x),
        }
        fmt.dedent();
    }

    pub fn get_type_name(&self, type_id: TypeId) -> String {
        let ty = &self.context.symbols[type_id];
        let ty_indent_id = match &ty {
            Type::Unknown => IdentId::TYPE_UNKNOWN,
            Type::Struct(CheckedStructNode { name, .. }) => *name,
            Type::Enum(CheckedEnumNode { name, .. }) => *name,
            Type::Function(CheckedFunctionNode { name, .. }) => *name,
            Type::Trait(CheckedTraitNode { name, .. }) => *name,
            Type::Const(node) => match node.name {
                Some(name) => name,
                None => return self.get_type_name(node.ty),
            },
            Type::Array(_) => IdentId::TYPE_ARRAY,
            Type::VOID => IdentId::TYPE_VOID,
            Type::Felt(_) => IdentId::TYPE_FELT,
            Type::Bool(_) => IdentId::TYPE_BOOL,
            Type::U32(_) => IdentId::TYPE_U32,
            Type::Tuple(_) => IdentId::TYPE_TUPLE,
            Type::TypeVariable(_) => {
                return "TypeVariable".to_string();
            }
            Type::LambdaFunction(_) => {
                return "LambdaFunction".to_string();
            }
            Type::FunctionSignature(_) => {
                return "FunctionSignature".to_string();
            }
            Type::GenericInstance(type_id, type_ids, _) => match &self.context.symbols[*type_id] {
                Type::Array(_) => {
                    let t = self.get_type_name(type_ids[0]);
                    let n = self.get_type_name(type_ids[1]);
                    return format!("[{}; {}]", t, n);
                }
                Type::Struct(_) => return self.get_type_name(*type_id),
                _ => {
                    unimplemented!()
                }
            },
        };
        let type_name = &self.context.ident(ty_indent_id).0;
        type_name.to_string()
    }

    pub fn fmt_type_names(&self, attr: &str, values: &[TypeId], fmt: &mut IndentFormatter) {
        if !values.is_empty() {
            let implementations = values
                .iter()
                .map(|ty| self.get_type_name(*ty))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(fmt, "{}: {}", attr, implementations);
        }
    }

    pub fn debug_variable_inline(
        &self,
        ident_id: IdentId,
        var_id: VarId,
        fmt: &mut IndentFormatter,
    ) {
        let var_name = &self.context.ident(ident_id).0;
        let checked_var = &self.context.symbols[var_id];
        fmt.write_indent();
        if checked_var.qualifier.is_mutable {
            write!(fmt, "mut ");
        };
        let type_name = self.get_type_name(checked_var.ty);
        write!(fmt, "{}: {}", var_name, type_name);
        write!(fmt, "\n")
    }
}

impl<F: Clone + From<u32> + ContextFelt, C> AstVisualizer<F, C>
    for TypeCheckerVisitorContext<F, C>
{
    type DebugResult = String;
    fn debug_scope(&self, scope_id: ScopeId) -> Self::DebugResult {
        let visualizer = TypeCheckerVisitorVisualizerInner { context: &self };
        let mut fmt = IndentFormatter::new();
        visualizer.debug_scope(scope_id, &mut fmt);
        fmt.finish()
    }

    fn debug_type(&self, type_id: TypeId) -> Self::DebugResult {
        let visualizer = TypeCheckerVisitorVisualizerInner { context: &self };
        let mut fmt = IndentFormatter::new();
        visualizer.debug_type(type_id, &mut fmt);
        fmt.finish()
    }

    fn debug_variable(&self, ident_id: IdentId, var_id: VarId) -> Self::DebugResult {
        let visualizer = TypeCheckerVisitorVisualizerInner { context: &self };
        let mut fmt = IndentFormatter::new();
        visualizer.debug_variable_inline(ident_id, var_id, &mut fmt);
        fmt.finish()
    }
}
