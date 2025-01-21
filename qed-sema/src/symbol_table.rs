use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::{Display, Formatter},
    hash::Hash,
    ops::{Index, IndexMut},
};

use qed_common::FileId;
use strum::{EnumIs, EnumTryAs};

use crate::{
    variable::CheckedVariable, CheckedFunctionNode, DefinitionNode, IdentId, ModuleId, ModuleKind,
    Type, TypeId, TypeKey, UseKind, UsePath,
};
use crate::{Error, Result};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

impl ScopeId {
    pub const fn root() -> Self {
        Self(0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScopeKind {
    Module,
    Block,
    Function,
    Struct,
    Enum,
    Impl,
    ImplMethod,
}

#[derive(Clone, Debug)]
pub struct Scope<T> {
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub children: Vec<ScopeId>,
    pub variables: HashMap<IdentId, CheckedVariable<T>>,
    pub types: HashMap<TypeKey, TypeId>,
}

#[derive(Clone, Debug)]
pub struct Module {
    pub name: IdentId,
    pub id: ModuleId,
    pub scope_id: ScopeId,
    pub kind: ModuleKind,
    pub parent: Option<ModuleId>,
    pub children: Vec<ModuleId>,
}

impl Module {
    pub fn new(
        name: IdentId,
        id: ModuleId,
        scope_id: ScopeId,
        file_id: FileId,
        parent: Option<ModuleId>,
    ) -> Self {
        Self {
            name,
            id,
            scope_id,
            kind: ModuleKind::File {
                file_id,
                is_dir: false,
            },
            parent,
            children: vec![],
        }
    }
}

impl<T> Scope<T> {
    pub fn new(kind: ScopeKind, parent: Option<ScopeId>) -> Self {
        Self {
            kind,
            parent,
            children: vec![],
            variables: HashMap::with_capacity(10),
            types: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SymbolTable<T> {
    scopes: Vec<Scope<T>>,
    scope_stack: Vec<ScopeId>,

    types: Vec<Type>,
    modules: Vec<Module>,
    module_stack: Vec<ModuleId>,
}

macro_rules! impl_index {
    ($index_type:ty, $output_type:ty, $field:ident) => {
        impl<T> Index<$index_type> for SymbolTable<T> {
            type Output = $output_type;
            fn index(&self, index: $index_type) -> &Self::Output {
                &self.$field[index.0]
            }
        }

        impl<T> IndexMut<$index_type> for SymbolTable<T> {
            fn index_mut(&mut self, index: $index_type) -> &mut Self::Output {
                &mut self.$field[index.0]
            }
        }
    };
}

impl_index!(ModuleId, Module, modules);
impl_index!(TypeId, Type, types);
impl_index!(ScopeId, Scope<T>, scopes);

impl<T> Display for SymbolTable<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, scope) in self.scopes.iter().enumerate() {
            writeln!(f, "ScopeId({})", i)?;
            writeln!(f, "  kind: {:?}", scope.kind)?;
            writeln!(f, "  parent: {:?}", scope.parent)?;
            writeln!(f, "  children: {:?}", scope.children)?;
            writeln!(f, "  types:")?;
            //print scope type
            for (k,v) in &scope.types {
                writeln!(f, "  {:?} : {:?}", k, v)?;
            }
        }
        //print type
        for (i, ty) in self.types.iter().enumerate() {
            writeln!(f, "TypeId({})", i)?;
            writeln!(f, "  {:?}", ty)?;
        }
        //print module
        for (i, module) in self.modules.iter().enumerate() {
            writeln!(f, "ModuleId({})", i)?;
            writeln!(f, "  name: {:?}", module.name)?;
            writeln!(f, "  id: {:?}", module.id)?;
            writeln!(f, "  scope_id: {:?}", module.scope_id)?;
            writeln!(f, "  kind: {:?}", module.kind)?;
            writeln!(f, "  parent: {:?}", module.parent)?;
            writeln!(f, "  children: {:?}", module.children)?;
        }
        Ok(())
    }
}

impl<T> SymbolTable<T> {
    pub fn new() -> Self {
        SymbolTable {
            scopes: vec![],
            scope_stack: vec![],

            types: vec![],
            modules: vec![],
            module_stack: vec![],
        }
    }

    pub fn current_scope_id(&self) -> Option<ScopeId> {
        self.scope_stack.last().cloned()
    }

    pub fn parent_scope_id(&self) -> Option<ScopeId> {
        self[self.current_scope_id()?].parent
    }

    pub fn current_module_id(&self) -> Option<ModuleId> {
        self.module_stack.last().cloned()
    }

    pub fn start_existing_module(&mut self, module_id: ModuleId) {
        self.scope_stack.push(self[module_id].scope_id);

        let current_module_id = self.current_module_id();
        self.module_stack.push(module_id);

        if let Some(current_module_id) = current_module_id {
            self[current_module_id].children.push(module_id);
        }
    }

    pub fn start_module(&mut self, name: IdentId, file_id: FileId) {
        let scope_id = ScopeId(self.scopes.len());
        self.scopes
            .push(Scope::new(ScopeKind::Module, self.current_scope_id()));
        self.scope_stack.push(scope_id);

        let current_module_id = self.current_module_id();
        let module_id = ModuleId(self.modules.len());

        self.module_stack.push(module_id);
        self.modules.push(Module::new(
            name,
            module_id,
            scope_id,
            file_id,
            current_module_id,
        ));

        if let Some(current_module_id) = current_module_id {
            self[current_module_id].children.push(module_id);
        }
    }

    pub fn end_module(&mut self) {
        self.scope_stack.pop();
        self.module_stack.pop();
    }

    pub fn add_type_id<K: Into<TypeKey>>(
        &mut self,
        scope_id: Option<ScopeId>,
        type_name: K,
        type_id: TypeId,
    ) {
        let type_name = type_name.into();
        let scope_id = scope_id.or(self.current_scope_id()).unwrap();

        if let Some(id) = self[scope_id].types.get(&type_name) {
            return;
        }

        self[scope_id].types.insert(type_name, type_id);
    }

    pub fn add_type(&mut self, scope_id: Option<ScopeId>, ty: Type) -> TypeId {
        let key = ty.key();
        if let Some(type_id) = self.get_type_id(scope_id, key.clone()) {
            return type_id;
        }

        let type_id = TypeId(self.types.len());
        self.add_type_id(scope_id, key, type_id);
        self.types.push(ty);
        type_id
    }

    pub fn add_type_variable(&mut self, ty: IdentId) -> TypeId {
        let type_id = TypeId(self.types.len());
        self.types.push(Type::TypeVariable(ty));
        self.add_type_id(None, ty, type_id);
        type_id
    }

    pub fn add_use(&mut self, use_path: &UsePath) -> Result<()> {
        let current_scope_id = self.current_scope_id().unwrap();
        eprintln!(
            "DEBUGPRINT[3]: symbol_table.rs:214: use_path={:#?}",
            use_path
        );
        let type_ids = self
            .resolve_use(&use_path)
            .ok_or(Error::UnresolvedUse)?
            .into_iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();
        for (ident, type_id) in type_ids {
            self.add_type_id(None, ident, type_id);
        }
        Ok(())
    }

    pub fn resolve_implementor(&mut self, ty: IdentId) -> Result<(ScopeId, TypeId)> {
        let current_scope_id = self.current_scope_id().unwrap();
        if let Some(&type_id) = self[current_scope_id].types.get(&ty.into()) {
            let scope_id = match &self[type_id] {
                Type::Struct(x) => x.scope_id,
                Type::Enum(x) => x.scope_id,
                _ => todo!(),
            };
            return Ok((scope_id, type_id));
        }
        Err(Error::UnresolvedImplementor)
    }

    pub fn resolve_method(&self, scope_id: ScopeId, method_name: IdentId) -> Option<TypeId> {
        let method_name: TypeKey = method_name.into();
        for &scope in &self[scope_id].children {
            if self[scope].kind == ScopeKind::Impl {
                for &scope in &self[scope].children {
                    if let Some(&type_id) = self[scope].types.get(&method_name) {
                        match &self[type_id] {
                            Type::Function(f) => return Some(type_id),
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
        None
    }
    pub fn resolve_method_with_path(
        &self,
        scope_id: ScopeId,
        method_name: IdentId,
    ) -> Option<Vec<(TypeId, Vec<ScopeId>)>> {
        let method_name: TypeKey = method_name.into();
        let res = self.find_type_recursive(scope_id, &method_name);
        if res.is_empty() {
            println!("cannot find type of {:?}", scope_id);
            return None;
        }
        let r = res.iter().map(|x| {
            let type_id = self[*x].types.get(&method_name).cloned();
            (type_id.unwrap(), res.clone())
        }).collect::<Vec<_>>();
        //println!("symbol =\n{}", self);

        // for i in &r {
        //     println!("{}:{}", file!(), line!());
        //     println!("i = {:?}", i);
        // }
        if r.is_empty(){
            return None;
        }else {
            return Some(r);
        }
    }
    pub fn find_module(&self, name: IdentId) -> Option<ModuleId> {
        self.modules.iter().position(|x| x.name == name).map(ModuleId)
    }
    pub fn resolve_use(&self, use_path: &UsePath) -> Option<Vec<(&TypeKey, &TypeId)>> {
        eprintln!(
            "DEBUGPRINT[8]: symbol_table.rs:260: self.modules={:#?}",
            self.modules
        );
        let mut src_module = match use_path.kind {
            UseKind::MODULE(name) => ModuleId(self.modules.iter().position(|x| x.name == name)?),
            UseKind::SELF => self.current_module_id()?,
            UseKind::CRATE => {
                let mut module_id = self.current_module_id()?;
                while let Some(parent) = self[module_id].parent {
                    module_id = parent;
                }
                module_id
            }
            UseKind::SUPER => {
                let module_id = self.current_module_id()?;
                self[module_id].parent?
            }
        };

        let mut path = use_path.segments.iter();
        while let Some(segment) = path.next() {
            eprintln!(
                "DEBUGPRINT[6]: symbol_table.rs:277: self[src_module].children={:#?}",
                self[src_module].children
            );
            eprintln!(
                "DEBUGPRINT[7]: symbol_table.rs:278: src_module={:#?}",
                src_module
            );
            let target_module_id = self[src_module].children.iter().find(|&id| {
                let module = &self[*id];
                module.name == *segment
            })?;
            src_module = *target_module_id;
        }

        if let Some(target) = use_path.target {
            self[self[src_module].scope_id]
                .types
                .get_key_value(&target.into())
                .map(|x| vec![x])
        } else {
            Some(
                self[self[src_module].scope_id]
                    .types
                    .iter()
                    .collect::<Vec<_>>(),
            )
        }
    }

    pub fn push_module(&mut self, module_id: ModuleId) {
        self.module_stack.push(module_id);
    }

    pub fn pop_module(&mut self) {
        self.module_stack.pop();
    }

    pub fn push_scope(&mut self, scope_id: ScopeId) {
        self.scope_stack.push(scope_id);
    }

    pub fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    pub fn start_scope(&mut self, kind: ScopeKind) {
        let current_scope_id = self.current_scope_id().unwrap();
        let child_scope_id = ScopeId(self.scopes.len());
        self.scopes.push(Scope::new(kind, Some(current_scope_id)));
        self[current_scope_id].children.push(child_scope_id);
        self.scope_stack.push(child_scope_id);
    }

    pub fn end_scope(&mut self) {
        self.scope_stack.pop();
    }

    pub fn start_function(&mut self) {
        self.start_scope(ScopeKind::Function);
    }

    pub fn end_function(&mut self) {
        self.end_scope();
    }

    pub fn get_type_id<S: Into<TypeKey>>(
        &self,
        start_scope: Option<ScopeId>,
        name: S,
    ) -> Option<TypeId> {
        let name: TypeKey = name.into();
        let scope_id = self.find_scope(start_scope, vec![ScopeKind::Module], |scope| {
            scope.types.contains_key(&name)
        })?;
        self[scope_id].types.get(&name).cloned()
    }
    pub fn get_type_id_with_scope<S: Into<TypeKey>>(
        &self,
        start_scope: Option<ScopeId>,
        name: S,
    ) -> Option<TypeId> {
        let name: TypeKey = name.into();
        let scope_id = self.find_scope(start_scope, vec![ScopeKind::Module], |scope| {
            scope.types.contains_key(&name)
        })?;
        self[scope_id].types.get(&name).cloned()
    }

    pub fn get_function_type(&self, idx: IdentId) -> Vec<CheckedFunctionNode> {
        let ret = self.types.iter().filter_map(|i| match i {
            Type::Function(f) => {
                if f.name == idx {
                    Some(f.clone())
                } else {
                    None
                }
            }
            _ => None,
        }).collect::<Vec<_>>();
        ret
    }
    pub fn search_type_table(&self, idx: IdentId) -> Vec<Type> {
        //todo use macro to optimize below code
        let ret = self.types.iter().filter_map(|i| match i {
            Type::Function(f) => {
                if f.name == idx {
                    Some(Type::Function(f.clone()))
                } else {
                    None
                }
            }
            Type::Struct(s) => {
                if s.name == idx {
                    Some(Type::Struct(s.clone()))
                } else {
                    None
                }
            }
            Type::Enum(e) => {
                if e.name == idx {
                    Some(Type::Enum(e.clone()))
                } else {
                    None
                }
            }
            _ => None,
        }).collect::<Vec<_>>();
        ret
    }


    fn find_scope(
        &self,
        start_scope: Option<ScopeId>,
        scope_kinds: Vec<ScopeKind>,
        f: impl Fn(&Scope<T>) -> bool,
    ) -> Option<ScopeId> {
        let mut current_scope_id = start_scope.or(self.current_scope_id());

        while let Some(scope_id) = current_scope_id {
            if f(&self[scope_id]) {
                return Some(scope_id);
            }
            if scope_kinds.iter().any(|x| x == &self[scope_id].kind) {
                return None;
            }
            current_scope_id = self[scope_id].parent;
        }

        return None;
    }

    fn find_type_recursive(
        &self,
        start_scope: ScopeId,
        type_key: &TypeKey,
    ) -> Vec<ScopeId> {
        let mut current_scope_id = start_scope;

        let mut r = self[current_scope_id].children.iter().map(|x| {
            self.find_type_recursive(*x, type_key)
        })
            .filter(|x| !x.is_empty())
            .flatten()
            .collect::<Vec<_>>();

        if self[current_scope_id].types.contains_key(type_key) {
            r.push(current_scope_id);
        }
        r

    }
    pub fn find_path_scope(
        &self,
        path_vec: &Vec<IdentId>,
        start_scope: ScopeId,
    ) -> Vec<ScopeId> {
        //if path_vec is empty, return the start_scope
        if path_vec.is_empty(){
            return vec![start_scope];
        }

        let mut idx = 0 ;
        let mut scopes = self.find_type_recursive(start_scope, &path_vec[idx].into());

        if scopes.is_empty() {
            return vec![];
        }
        
        'outer: loop {

            idx += 1;

            if idx == path_vec.len(){
               break 'outer;
            }
            let mut new_scopes = vec![];
            'inner: loop {
                let s = match scopes.pop(){
                    Some(s) => s,
                    None => break 'inner,
                };
                let ss = self.find_type_recursive(s, &path_vec[idx].into());
                new_scopes.extend(ss);
            }
            if new_scopes.is_empty(){
                break 'outer;
            }
            scopes = new_scopes;

        }

        scopes
    }
    pub fn get_scope(
        &self,
        start_scope: Option<ScopeId>,
        target: &IdentId,
    )-> Option<ScopeId>{
        let type_key: TypeKey = (*target).into();
        let scope_id = self.find_scope(
            start_scope,
            //note: maybe should be fewer options below
            vec![],//use none so that it will search all the way
            |scope| {
            scope.types.contains_key(&type_key)
        });
        scope_id
    }
    pub fn get_variable(
        &mut self,
        start_scope: Option<ScopeId>,
        key: &IdentId,
    ) -> Option<&mut CheckedVariable<T>> {
        let scope_id = self.find_scope(
            start_scope,
            vec![ScopeKind::Function, ScopeKind::ImplMethod],
            |scope| scope.variables.contains_key(key),
        )?;
        self[scope_id].variables.get_mut(key)
    }
    pub fn find_variable_scope(
        &mut self,
        start_scope: Option<ScopeId>,
        key: &IdentId,
    ) -> Option<ScopeId> {
        let scope_id = self.find_scope(
            start_scope,
            vec![ScopeKind::Function, ScopeKind::ImplMethod],
            |scope| scope.variables.contains_key(key),
        );

        scope_id

    }
    pub fn set_variable(
        &mut self,
        start_scope: Option<ScopeId>,
        key: &IdentId,
        value: T,
    ) -> Result<()> {
        if let Some(v) = self
            .find_scope(
                start_scope,
                vec![ScopeKind::Function, ScopeKind::ImplMethod],
                |scope| scope.variables.contains_key(key),
            )
            .and_then(|scope_id| self[scope_id].variables.get_mut(key))
        {
            if v.value.is_some() && (!v.mutable || v.cnst) {
                return Err(Error::ImmutableVariable);
            }

            v.value = Some(value);
            return Ok(());
        }

        Err(Error::UndefinedVariable)
    }

    pub fn define_variable(&mut self, key: IdentId, value: CheckedVariable<T>) -> Result<()> {
        let current_scope_id = self.current_scope_id().unwrap();
        if self[current_scope_id].variables.contains_key(&key) {
            return Err(Error::VariableAlreadyDefined);
        }
        self[current_scope_id].variables.insert(key, value);
        Ok(())
    }

    pub fn get_types_len(&self) -> usize {
        self.types.len()
    }
    
}

#[cfg(test)]
mod tests {
    use smol_str::SmolStr;

    use super::*;

    pub fn value(n: i32) -> CheckedVariable<i32> {
        CheckedVariable {
            ty: TypeId(0),
            mutable: true,
            cnst: false,
            scope_id: ScopeId(0),
            value: Some(n),
        }
    }

    #[test]
    fn test_new_symbol_table() {
        let mut table: SymbolTable<i32> = SymbolTable::new();
        table.start_module(IdentId(0), FileId(0));
        assert_eq!(table.scope_stack.len(), 1);
        table.end_module();
    }

    #[test]
    fn test_start_and_end_scope() {
        let mut table: SymbolTable<i32> = SymbolTable::new();
        table.start_module(IdentId(0), FileId(0));
        table.start_scope(ScopeKind::Block);
        assert_eq!(table.scope_stack.len(), 2);
        table.end_scope();
        assert_eq!(table.scope_stack.len(), 1);
        table.end_module();
    }

    #[test]
    fn test_start_and_end_function() {
        let mut table: SymbolTable<i32> = SymbolTable::new();
        table.start_module(IdentId(0), FileId(0));
        table.start_function();
        assert_eq!(table.scope_stack.len(), 2);
        table.end_function();
        assert_eq!(table.scope_stack.len(), 1);
        table.end_module();
    }

    #[test]
    fn test_scope_set_and_get() {
        let mut table: SymbolTable<i32> = SymbolTable::new();
        table.start_module(IdentId(0), FileId(0));
        table.start_scope(ScopeKind::Block);
        table.define_variable(IdentId(0), value(42));
        assert_eq!(table.get_variable(None, &IdentId(0)), Some(&mut value(42)));

        table.start_scope(ScopeKind::Block);
        assert_eq!(table.get_variable(None, &IdentId(0)), Some(&mut value(42)));
        table.define_variable(IdentId(0), value(24));
        assert_eq!(table.get_variable(None, &IdentId(0)), Some(&mut value(24)));
        table.end_scope();
        assert_eq!(table.get_variable(None, &IdentId(0)), Some(&mut value(42)));
        table.end_scope();
        table.end_module();
    }

    #[test]
    fn test_function_set_and_get() {
        let mut table: SymbolTable<i32> = SymbolTable::new();
        table.start_module(IdentId(0), FileId(0));
        table.start_function();
        table.define_variable(IdentId(0), value(42));
        assert_eq!(table.get_variable(None, &IdentId(0)), Some(&mut value(42)));
        table.start_function();
        assert_eq!(table.get_variable(None, &IdentId(0)), None);
        table.set_variable(None, &IdentId(0), 24);
        assert_eq!(table.get_variable(None, &IdentId(0)), None);
        table.end_function();
        assert_eq!(table.get_variable(None, &IdentId(0)), Some(&mut value(42)));
        table.end_function();
        table.end_module();
    }

    #[test]
    fn test_global_value() {
        let mut table: SymbolTable<i32> = SymbolTable::new();
        table.start_module(IdentId(0), FileId(0));
        table.define_variable(IdentId(0), value(42));
        table.start_scope(ScopeKind::Block);
        assert_eq!(table.get_variable(None, &IdentId(0)), Some(&mut value(42)));
        table.end_scope();
        table.start_function();
        assert_eq!(table.get_variable(None, &IdentId(0)), None);
        table.define_variable(IdentId(0), value(42));
        table.set_variable(None, &IdentId(0), 24);
        assert_eq!(table.get_variable(None, &IdentId(0)), Some(&mut value(24)));
        table.end_function();
        assert_eq!(table.get_variable(None, &IdentId(0)), Some(&mut value(42)));
        table.end_module();
    }
}
