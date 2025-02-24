use std::{
    collections::HashMap,
    convert::AsMut,
    fmt::{Display, Formatter},
    hash::Hash,
    iter::once,
    ops::{Index, IndexMut},
};

use once_cell::sync::OnceCell;
use qed_ast::{ModuleNode, PathNode, Visibility};
use qed_common::{define_arena_id, FileId, TreeNode};

use crate::{
    variable::CheckedVariable, CheckedTraitNode, CheckedValueRef, IdentId, ModuleId, ModuleKind,
    Type, TypeId, TypeKey, UsePath,
};
use crate::{Error, Result};

define_arena_id!(ScopeId);
define_arena_id!(VarId);

pub static mut STD_PRIMITIVE_SCOPE_ID: OnceCell<ScopeId> = OnceCell::new();

impl ScopeId {
    pub const fn root() -> Self {
        Self(0)
    }

    pub fn primitive() -> Self {
        #[allow(static_mut_refs)]
        unsafe {
            *STD_PRIMITIVE_SCOPE_ID.get().unwrap()
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ScopeKind {
    Module,
    Block,
    Function,
    Struct,
    StructInstance,
    Enum,
    Impl,
    ImplMethod,
    Trait,
    TraitMethod,
}

#[derive(Clone, Debug)]
pub struct Scope<F: Clone> {
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub children: Vec<ScopeId>,
    pub variables: HashMap<IdentId, VarId>,
    pub types: HashMap<TypeKey, TypeId>,
    _marker: std::marker::PhantomData<F>,
}

#[derive(Clone, Debug)]
pub struct Frame<T: Clone> {
    pub variables: Vec<(ScopeId, HashMap<IdentId, T>)>,
}

impl<T: Clone> Frame<T> {
    pub fn new(scope_id: ScopeId) -> Self {
        Self {
            variables: vec![(scope_id, HashMap::new())],
        }
    }

    pub fn push_scope(&mut self, scope_id: ScopeId) {
        self.variables.push((scope_id, HashMap::new()));
    }

    pub fn pop_scope(&mut self) {
        self.variables.pop();
    }

    pub fn set_value(&mut self, scope_id: ScopeId, key: IdentId, value: T) {
        for (sid, vars) in self.variables.iter_mut().rev() {
            if *sid == scope_id {
                vars.insert(key, value);
                return;
            }
        }
    }

    pub fn get_value(&self, scope_id: ScopeId, key: &IdentId) -> Option<&T> {
        for (sid, vars) in self.variables.iter().rev() {
            if *sid == scope_id {
                return vars.get(key);
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct Module {
    pub name: IdentId,
    pub id: ModuleId,
    pub scope_id: ScopeId,
    pub kind: ModuleKind,
    pub parent: Option<ModuleId>,
    pub children: Vec<ModuleId>,
    pub visibility: Visibility,
}

impl Module {
    pub fn new(
        name: IdentId,
        id: ModuleId,
        scope_id: ScopeId,
        file_id: FileId,
        parent: Option<ModuleId>,
        visibility: Visibility,
    ) -> Self {
        Self {
            name,
            id,
            scope_id,
            kind: ModuleKind::File { file_id },
            parent,
            children: vec![],
            visibility,
        }
    }
}

impl<F: Clone> Scope<F> {
    pub fn new(kind: ScopeKind, parent: Option<ScopeId>) -> Self {
        Self {
            kind,
            parent,
            children: vec![],
            variables: HashMap::with_capacity(10),
            types: HashMap::new(),
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SymbolTable<F: Clone> {
    scopes: Vec<Scope<F>>,
    scope_stack: Vec<ScopeId>,
    frames: Vec<Frame<CheckedValueRef<F>>>,

    types: Vec<Type>,
    variables: Vec<CheckedVariable<F>>,
    modules: Vec<Module>,
    module_stack: Vec<ModuleId>,
}

macro_rules! impl_index {
    ($index_type:ty, $output_type:ty, $field:ident) => {
        impl<F: Clone> Index<$index_type> for SymbolTable<F> {
            type Output = $output_type;
            fn index(&self, index: $index_type) -> &Self::Output {
                &self.$field[index.0]
            }
        }

        impl<F: Clone> IndexMut<$index_type> for SymbolTable<F> {
            fn index_mut(&mut self, index: $index_type) -> &mut Self::Output {
                &mut self.$field[index.0]
            }
        }
    };
}

impl_index!(ModuleId, Module, modules);
impl_index!(TypeId, Type, types);
impl_index!(VarId, CheckedVariable<F>, variables);
impl_index!(ScopeId, Scope<F>, scopes);

impl<T: Clone> Display for SymbolTable<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, scope) in self.scopes.iter().enumerate() {
            writeln!(f, "ScopeId({})", i)?;
            writeln!(f, "  kind: {:?}", scope.kind)?;
            writeln!(f, "  parent: {:?}", scope.parent)?;
            writeln!(f, "  children: {:?}", scope.children)?;
            writeln!(f, "  types:")?;
            //print scope type
            for (k, v) in &scope.types {
                writeln!(f, "  {:?} : {:?}", k, v)?;
            }
            //variables
            writeln!(f, "  variables:")?;
            for (k, v) in &scope.variables {
                writeln!(f, "  {:?} : {:?}", k, v)?;
            }
        }
        //print type
        for (i, ty) in self.types.iter().enumerate() {
            writeln!(f, "TypeId({}) ： {:?}", i, ty)?;
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

impl<F: Clone> SymbolTable<F> {
    pub fn new() -> Self {
        SymbolTable {
            scopes: vec![],
            scope_stack: vec![],
            frames: vec![],

            types: vec![],
            variables: vec![],
            modules: vec![],
            module_stack: vec![],
        }
    }

    pub fn load_modules<'a>(
        &mut self,
        modules: impl IntoIterator<Item = &'a TreeNode<ModuleId, ModuleNode>>,
    ) {
        for module in modules {
            let data = module.data();
            self.modules.push(Module {
                name: data.name.clone(),
                id: module.id(),
                scope_id: ScopeId(module.id().into()),
                kind: ModuleKind::File {
                    file_id: data.file_id,
                },
                parent: module.parent(),
                children: module.children().to_vec(),
                visibility: data.visibility,
            });
            self.scopes.push(Scope {
                kind: ScopeKind::Module,
                parent: module.parent().map(|x| ScopeId(x.into())),
                children: module
                    .children()
                    .into_iter()
                    .map(|&x| ScopeId(x.into()))
                    .collect(),
                variables: HashMap::with_capacity(10),
                types: HashMap::new(),
                _marker: std::marker::PhantomData,
            })
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

    pub fn add_type_id<K: Into<TypeKey>>(
        &mut self,
        scope_id: Option<ScopeId>,
        name: K,
        type_id: TypeId,
    ) -> Result<()> {
        let key = name.into();
        let scope_id = scope_id.or(self.current_scope_id()).unwrap();

        if let Some(type_id) = self[scope_id].types.get(&key) {
            return Err(Error::TypeAlreadyDefined);
        }

        self[scope_id].types.insert(key, type_id);
        Ok(())
    }

    pub fn add_type<K: Into<TypeKey>>(
        &mut self,
        scope_id: Option<ScopeId>,
        name: K,
        ty: Type,
    ) -> Result<TypeId> {
        let key = name.into();
        let type_id = TypeId(self.types.len());
        self.add_type_id(scope_id, key, type_id)?;
        self.types.push(ty);
        Ok(type_id)
    }

    pub fn get_or_add_type<K: Into<TypeKey>>(
        &mut self,
        scope_id: Option<ScopeId>,
        name: K,
        ty: Type,
    ) -> Result<TypeId> {
        let key = name.into();
        let scope_id = scope_id.or(self.current_scope_id());
        if let Some(type_id) = self.get_type_id(scope_id, ty.key()) {
            Ok(type_id)
        } else {
            let type_id = TypeId(self.types.len());
            self[scope_id.unwrap()].types.insert(key, type_id);
            self.types.push(ty);
            Ok(type_id)
        }
    }

    pub fn add_type_variable(&mut self, end_scope_kind: ScopeKind, ty: IdentId) -> Result<TypeId> {
        if let Some(type_id) = self.get_type_variable(None, end_scope_kind, ty) {
            return Ok(type_id);
        }

        let type_id = TypeId(self.types.len());
        self.types.push(Type::TypeVariable(ty));
        self.add_type_id(None, ty, type_id)?;
        Ok(type_id)
    }

    pub fn add_use(&mut self, use_path: &UsePath) -> Result<()> {
        let type_ids = self.resolve_use(&use_path).ok_or(Error::UnresolvedUse)?;
        let type_ids = type_ids
            .into_iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();
        for (mut key, type_id) in type_ids {
            key.visibility = use_path.visibility;
            let _ = self.add_type_id(None, key.clone(), type_id);
        }
        Ok(())
    }

    pub fn resolve_method(&self, implementor_id: TypeId, method_name: IdentId) -> Option<TypeId> {
        let method_name_key: TypeKey = method_name.into();

        let find_method = |type_id: TypeId| -> Option<TypeId> {
            let scope_id = self[type_id].scope_id();
            let trait_type_id = type_id;
            let is_trait = self[type_id].is_trait();

            for &impl_scope in &self[scope_id].children {
                if self[impl_scope].kind == ScopeKind::Impl {
                    for &fun_scope in &self[impl_scope].children {
                        if let Some(&type_id) = self[fun_scope].types.get(&method_name_key) {
                            if !self[type_id].is_function() {
                                continue;
                            }
                            if !is_trait {
                                return Some(type_id);
                            }
                            if self[impl_scope].types.get(&IdentId::TYPE_SELF.into())
                                == Some(&implementor_id)
                                && self.get_type_id(None, self[trait_type_id].key()).is_some()
                            {
                                return Some(type_id);
                            }
                        }
                    }
                }
            }

            None
        };

        for &type_id in
            once(&implementor_id).chain(self[implementor_id].implementations().into_iter())
        {
            if let Some(type_id) = find_method(type_id) {
                return Some(type_id);
            }
        }

        None
    }

    pub fn resolve_use(&self, use_path: &UsePath) -> Option<Vec<(&TypeKey, &TypeId)>> {
        let current_module_id = self.current_module_id()?;

        let mut src_module = match use_path.kind {
            IdentId::SELF => current_module_id,
            IdentId::CRATE => {
                let mut module_id = current_module_id;
                while let Some(parent) = self[module_id].parent {
                    module_id = parent;
                }
                module_id
            }
            IdentId::SUPER => self[current_module_id].parent?,
            name => {
                let module_id = ModuleId(self.modules.iter().position(|x| x.name == name)?);
                assert!(self[current_module_id].children.contains(&module_id));
                module_id
            }
        };

        let mut path = use_path.segments.iter();
        while let Some(segment) = path.next() {
            let target_module_id = self[src_module].children.iter().find(|&id| {
                let module = &self[*id];
                module.name == *segment
            })?;
            assert!(self[*target_module_id].visibility.is_public());
            src_module = *target_module_id;
        }

        if let Some(target) = use_path.target {
            let (key, type_id) = self[self[src_module].scope_id]
                .types
                .get_key_value(&target.into())
                .filter(|(key, _)| key.visibility.is_public())?;
            assert!(self[*type_id].visibility().is_public());
            Some(vec![(key, type_id)])
        } else {
            Some(
                self[self[src_module].scope_id]
                    .types
                    .iter()
                    .filter(|(key, &type_id)| {
                        key.visibility.is_public() && self[type_id].visibility().is_public()
                    })
                    .collect::<Vec<_>>(),
            )
        }
    }

    pub fn resolve_path(&self, path: &PathNode) -> Option<(TypeId, ScopeId)> {
        let current_module_id = self.current_module_id()?;

        let mut src_module = match path.root {
            Some(IdentId::SELF) => current_module_id,
            Some(IdentId::CRATE) => {
                let mut module_id = current_module_id;
                while let Some(parent) = self[module_id].parent {
                    module_id = parent;
                }
                module_id
            }
            Some(IdentId::SUPER) => self[current_module_id].parent?,
            Some(name) => {
                if let Some(&module_id) = self[current_module_id]
                    .children
                    .iter()
                    .find(|&x| self[x.clone()].name == name)
                {
                    module_id
                } else {
                    let type_id = self.get_type_id(None, name)?;
                    assert!(path.segments.is_empty());
                    let scope_id = self[type_id].scope_id();
                    if let Some(type_id) = self[scope_id].types.get(&path.target.into()) {
                        return Some((type_id.clone(), scope_id));
                    }
                    let method_type_id = self.resolve_method(type_id, path.target)?;
                    let visibility = self[method_type_id].visibility();
                    assert!(visibility.is_public());
                    return Some((method_type_id, self[method_type_id].scope_id()));
                }
            }
            None => {
                assert!(path.segments.is_empty());
                if let Some(variable) = self.get_variable(None, &path.target) {
                    return Some((variable.ty, variable.scope_id));
                } else {
                    let type_id = self.get_type_id(None, path.target)?;
                    return Some((type_id, self[type_id].scope_id()));
                };
            }
        };

        let mut segments = path.segments.iter();
        while let Some(segment) = segments.next() {
            if let Some(target_module_id) = self[src_module].children.iter().find(|&id| {
                let module = &self[*id];
                module.name == *segment
            }) {
                assert!(self[*target_module_id].visibility.is_public());
                src_module = *target_module_id;
            } else {
                assert!(segments.next().is_none());
                let type_id = self[self[src_module].scope_id]
                    .types
                    .get(&segment.clone().into())?
                    .clone();
                let visibility = self[type_id].visibility();
                assert!(visibility.is_public());
                let method_type_id = self.resolve_method(type_id, path.target)?;
                let visibility = self[method_type_id].visibility();
                assert!(visibility.is_public());
                return Some((method_type_id, self[method_type_id].scope_id()));
            }
        }

        let type_id = self[self[src_module].scope_id]
            .types
            .get(&path.target.clone().into())
            .cloned()?;
        let visibility = self[type_id].visibility();
        assert!(visibility.is_public());
        return Some((type_id, self[type_id].scope_id()));
    }

    pub fn impl_trait_for_type(&mut self, trait_type_id: TypeId, implementor: TypeId) {
        self[implementor].add_implementation(trait_type_id);

        (self[trait_type_id].as_mut() as &mut CheckedTraitNode).add_implementor(implementor);
    }

    pub fn enter_module(&mut self, module_id: ModuleId) {
        self.enter_scope(self[module_id].scope_id);
        self.module_stack.push(module_id);
    }

    pub fn exit_module(&mut self) {
        self.exit_scope();
        self.module_stack.pop();
    }

    pub fn enter_scope(&mut self, scope_id: ScopeId) {
        self.scope_stack.push(scope_id);
    }

    pub fn exit_scope(&mut self) {
        self.scope_stack.pop();
    }

    pub fn enter_function(&mut self, scope_id: ScopeId) {
        self.frames.push(Frame::new(scope_id));
    }

    pub fn exit_function(&mut self) {
        self.frames.pop();
    }

    pub fn enter_block(&mut self, scope_id: ScopeId) {
        self.frames.last_mut().unwrap().push_scope(scope_id);
    }

    pub fn exit_block(&mut self) {
        self.frames.last_mut().unwrap().pop_scope();
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

    pub fn get_type_id<K: Into<TypeKey>>(
        &self,
        start_scope: Option<ScopeId>,
        name: K,
    ) -> Option<TypeId> {
        let name: TypeKey = name.into();
        self.find(start_scope, vec![ScopeKind::Module], |scope| {
            scope.types.get(&name).cloned()
        })
    }

    pub fn get_type_variable<K: Into<TypeKey>>(
        &self,
        start_scope: Option<ScopeId>,
        end_scope_kind: ScopeKind,
        name: K,
    ) -> Option<TypeId> {
        let name: TypeKey = name.into();
        self.find(start_scope, vec![end_scope_kind], |scope| {
            scope.types.get(&name).cloned()
        })
    }

    pub fn find<R>(
        &self,
        start_scope: Option<ScopeId>,
        end_scope_kinds: Vec<ScopeKind>,
        f: impl Fn(&Scope<F>) -> Option<R>,
    ) -> Option<R> {
        let mut current_scope_id = start_scope.or(self.current_scope_id());

        while let Some(scope_id) = current_scope_id {
            if let Some(r) = f(&self[scope_id]) {
                return Some(r);
            }
            if end_scope_kinds.iter().any(|x| x == &self[scope_id].kind) {
                return None;
            }
            current_scope_id = self[scope_id].parent;
        }

        return None;
    }

    pub fn get_variable(
        &self,
        start_scope: Option<ScopeId>,
        key: &IdentId,
    ) -> Option<CheckedVariable<F>> {
        let var_id = self.find(
            start_scope,
            vec![
                ScopeKind::Function,
                ScopeKind::ImplMethod,
                ScopeKind::TraitMethod,
            ],
            |scope| scope.variables.get(key).cloned(),
        )?;

        let value = self
            .frames
            .last()
            .and_then(|frame| frame.get_value(self[var_id].scope_id, key))
            .cloned();

        let mut variable = self[var_id].clone();
        variable.value = value;
        Some(variable)
    }

    pub fn set_variable(
        &mut self,
        scope_id: ScopeId,
        key: &IdentId,
        value: CheckedValueRef<F>,
    ) -> Result<()> {
        if let Some(v) = self[scope_id].variables.get(key) {
            if self
                .frames
                .last()
                .unwrap()
                .get_value(scope_id, key)
                .is_some()
                && (!self[v.clone()].qualifier.is_mutable)
            {
                return Err(Error::ImmutableVariable);
            }

            self.frames
                .last_mut()
                .unwrap()
                .set_value(scope_id, key.clone(), value);
            return Ok(());
        }

        Err(Error::UndefinedVariable)
    }

    pub fn declare_variable(
        &mut self,
        key: IdentId,
        variable: CheckedVariable<F>,
    ) -> Result<VarId> {
        let scope_id = self.current_scope_id().unwrap();
        assert_eq!(variable.scope_id, scope_id);
        if self[scope_id].variables.contains_key(&key) {
            return Err(Error::VariableAlreadyDefined);
        }
        let var_id = VarId(self.variables.len());
        self.variables.push(variable);
        self[scope_id].variables.insert(key, var_id);
        Ok(var_id)
    }

    pub fn size_of(&self, type_id: TypeId) -> usize {
        match &self[type_id] {
            Type::Felt(f) => 1usize,
            Type::Bool(b) => 1usize,
            Type::Array(a) => self.size_of(a.inner_ty) * a.size,
            Type::Struct(s) => s
                .fields
                .iter()
                .map(|(_, (type_id, _))| self.size_of(type_id.clone()))
                .sum(),
            _ => unreachable!(),
        }
    }
}
