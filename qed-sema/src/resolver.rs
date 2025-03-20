use std::collections::{HashMap, HashSet};

use qed_ast::{
    IdentId, ImplTraitNode, Location, ModuleId, PathNode, UncheckedType, UseNode, VisitorContext,
};
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{
    AstVisualizer, CheckedPathNode, Error, Result, TypeChecker, TypeCheckerVisitorContext, TypeId,
    TypeKey,
};

#[derive(Debug)]
pub struct ResolverCtxt {
    impl_traits: HashMap<TypeId, HashSet<TypeId>>,
    trait_impls: HashMap<TypeId, HashSet<TypeId>>,

    impl_methods: HashMap<TypeId, HashMap<IdentId, TypeId>>,
    trait_methods: HashMap<TypeId, HashMap<TypeId, HashMap<IdentId, TypeId>>>,
}

impl ResolverCtxt {
    pub fn new() -> Self {
        Self {
            impl_traits: HashMap::new(),
            trait_impls: HashMap::new(),
            impl_methods: HashMap::new(),
            trait_methods: HashMap::new(),
        }
    }
}

pub trait Resolver<F: Clone + From<u32> + ContextFelt, C> {
    fn impl_trait_for_type(
        &mut self,
        impl_node: &ImplTraitNode,
        trait_type_id: TypeId,
        implementor_type_id: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()>;

    fn add_method(
        &mut self,
        location: &Location,
        ty: TypeId,
        method_name: IdentId,
        method: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()>;

    fn add_trait_method(
        &mut self,
        location: &Location,
        trait_type_id: TypeId,
        implementor_type_id: TypeId,
        method_name: IdentId,
        method: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()>;

    fn methods_of(
        &self,
        implementor: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Vec<TypeId>;

    fn resolve_method(
        &self,
        implementor_id: TypeId,
        method_name: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Option<TypeId>;

    fn resolve_path(
        &mut self,
        path: &PathNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedPathNode>;

    fn resolve_use(
        &self,
        use_path: &UseNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<Vec<(TypeKey, TypeId)>>;

    fn resolve_member_type(
        &mut self,
        path: &PathNode,
        root: TypeId,
        target: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId>;

    fn resolve_module_type(
        &mut self,
        path: &PathNode,
        module: ModuleId,
        ty: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId>;
}

impl<F: Clone + From<u32> + ContextFelt, C> Resolver<F, C> for TypeChecker<F, C> {
    fn impl_trait_for_type(
        &mut self,
        impl_node: &ImplTraitNode,
        trait_type_id: TypeId,
        implementor_type_id: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let location = impl_node.location;
        let pairs = [
            (
                &mut self.resolver.impl_traits,
                implementor_type_id,
                trait_type_id,
            ),
            (
                &mut self.resolver.trait_impls,
                trait_type_id,
                implementor_type_id,
            ),
        ];

        for (map, key, value) in pairs {
            if !map.entry(key).or_insert_with(HashSet::new).insert(value) {
                return Err(Error::TraitAlreadyImplemented {
                    location,
                    trait_ty: trait_type_id,
                    ty: implementor_type_id,
                });
            }
        }
        Ok(())
    }

    fn add_method(
        &mut self,
        location: &Location,
        ty: TypeId,
        method_name: IdentId,
        method: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let methods = self
            .resolver
            .impl_methods
            .entry(ty)
            .or_insert_with(HashMap::new);
        if methods.insert(method_name, method).is_some() {
            return Err(Error::DuplicatedMethod {
                location: location.clone(),
                method: method_name,
                ty: ty,
            });
        }
        Ok(())
    }

    fn add_trait_method(
        &mut self,
        location: &Location,
        trait_type_id: TypeId,
        implementor_type_id: TypeId,
        method_name: IdentId,
        method: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        if self
            .resolver
            .trait_impls
            .get(&trait_type_id)
            .map(|x| x.contains(&implementor_type_id))
            .unwrap_or(false)
        {
            return Err(Error::TraitAlreadyImplemented {
                location: location.clone(),
                trait_ty: trait_type_id,
                ty: implementor_type_id,
            });
        }

        let methods = self
            .resolver
            .trait_methods
            .entry(trait_type_id)
            .or_insert_with(HashMap::new)
            .entry(implementor_type_id)
            .or_insert_with(HashMap::new);
        if methods.insert(method_name, method).is_some() {
            return Err(Error::DuplicatedMethod {
                location: location.clone(),
                method: method_name,
                ty: implementor_type_id,
            });
        }
        Ok(())
    }

    fn methods_of(
        &self,
        implementor: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Vec<TypeId> {
        let mut all_methods = Vec::new();

        if let Some(methods) = self.resolver.impl_methods.get(&implementor) {
            all_methods.extend(methods.iter().map(|(_, &ty)| ty));
        }

        if let Some(trait_type_ids) = self.resolver.impl_traits.get(&implementor) {
            for &trait_type_id in trait_type_ids {
                if !ctx.is_trait_imported(trait_type_id) {
                    continue;
                }
                if let Some(trait_impls) = self.resolver.trait_methods.get(&trait_type_id) {
                    if let Some(trait_methods) = trait_impls.get(&implementor) {
                        all_methods.extend(trait_methods.iter().map(|(_, &ty)| ty));
                    }
                }
            }
        }

        all_methods
    }

    fn resolve_method(
        &self,
        implementor_id: TypeId,
        method_name: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Option<TypeId> {
        for type_id in self.methods_of(implementor_id, ctx) {
            if ctx.symbols[type_id].as_function().unwrap().name == method_name {
                return Some(type_id);
            }
        }

        None
    }

    fn resolve_path(
        &mut self,
        path: &PathNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedPathNode> {
        let current_module_id = ctx.symbols.current_module_id().unwrap();

        let mut src_module = match path.root.as_ref() {
            Some(UncheckedType::Basic(IdentId::SELF, _)) => current_module_id,
            Some(UncheckedType::Basic(IdentId::CRATE, _)) => {
                let mut module_id = current_module_id;
                while let Some(parent) = ctx.symbols[module_id].parent {
                    module_id = parent;
                }
                module_id
            }
            Some(UncheckedType::Basic(IdentId::SUPER, _)) => ctx.symbols[current_module_id]
                .parent
                .ok_or(Error::NoParentModule {
                    location: path.location,
                })?,
            Some(ty) => match ty {
                UncheckedType::Basic(name, _)
                    if let Some(&module_id) = ctx.symbols[current_module_id]
                        .children
                        .iter()
                        .find(|&x| &ctx.symbols[*x].name == name) =>
                {
                    module_id
                }
                _ => {
                    if !path.segments.is_empty() {
                        return Err(Error::InvalidPathSegment {
                            location: path.location,
                            segment: path.segments[0],
                        });
                    }
                    let root_type_id = self.typecheck(ty, ctx)?;
                    return Ok(CheckedPathNode::new(
                        None,
                        Some(root_type_id),
                        self.resolve_member_type(path, root_type_id, path.target, ctx)?,
                        path.location,
                    ));
                }
            },
            None => {
                if !path.segments.is_empty() {
                    return Err(Error::InvalidPathSegment {
                        location: path.location,
                        segment: path.segments[0],
                    });
                }
                if let Some(var_id) = ctx.symbols.get_variable(None, &path.target) {
                    return Ok(CheckedPathNode::new(
                        Some(var_id),
                        None,
                        ctx.symbols[var_id].ty,
                        path.location,
                    ));
                }
                let type_id = ctx.symbols.get_type_id(None, path.target).ok_or_else(|| {
                    Error::UnresolvedType {
                        location: path.location,
                        resolved_type: path.target,
                    }
                })?;
                return Ok(CheckedPathNode::new(None, None, type_id, path.location));
            }
        };

        for (i, segment) in path.segments.iter().enumerate() {
            if let Some(&target_module_id) = ctx.symbols[src_module]
                .children
                .iter()
                .find(|&&id| ctx.symbols[id].name == *segment)
            {
                if !ctx.symbols[target_module_id].visibility.is_public() {
                    return Err(Error::ModuleNotPublic {
                        location: path.location,
                        module: ctx.symbols[target_module_id].name,
                    });
                }
                src_module = target_module_id;
            } else {
                if i != path.segments.len() - 1 {
                    return Err(Error::InvalidPathSegment {
                        location: path.location,
                        segment: *segment,
                    });
                }
                let root_type_id =
                    self.resolve_module_type(path, src_module, segment.clone(), ctx)?;
                return Ok(CheckedPathNode::new(
                    None,
                    Some(root_type_id),
                    self.resolve_member_type(path, root_type_id, path.target, ctx)?,
                    path.location,
                ));
            }
        }

        let type_id = self.resolve_module_type(path, src_module, path.target, ctx)?;
        return Ok(CheckedPathNode::new(None, None, type_id, path.location));
    }

    fn resolve_use(
        &self,
        use_path: &UseNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<Vec<(TypeKey, TypeId)>> {
        let symbols = &ctx.symbols;
        let current_module_id = symbols.current_module_id().unwrap();

        let mut module_id = match use_path.kind {
            IdentId::SELF => current_module_id,
            IdentId::CRATE => {
                let mut root_id = current_module_id;
                while let Some(parent) = symbols[root_id].parent {
                    root_id = parent;
                }
                root_id
            }
            IdentId::SUPER => symbols[current_module_id]
                .parent
                .ok_or(Error::NoParentModule {
                    location: use_path.location,
                })?,
            name => symbols
                .modules()
                .iter()
                .position(|x| x.name == name)
                .map(ModuleId)
                .filter(|&id| symbols[current_module_id].children.contains(&id))
                .ok_or(Error::ModuleNotFound {
                    location: use_path.location,
                    module: name,
                })?,
        };

        module_id = use_path
            .segments
            .iter()
            .try_fold(module_id, |current, &segment| {
                if let Some(module) = symbols[current]
                    .children
                    .iter()
                    .find(|&&id| symbols[id].name == segment)
                    .copied()
                {
                    if ctx.symbols[module].visibility.is_public() {
                        Ok(module)
                    } else {
                        Err(Error::ModuleNotPublic {
                            location: use_path.location,
                            module: segment,
                        })
                    }
                } else {
                    Err(Error::ModuleNotFound {
                        location: use_path.location,
                        module: segment,
                    })
                }
            })?;

        let scope = &symbols[symbols[module_id].scope_id].types;
        match use_path.target {
            Some(target) => {
                if let Some((key, &type_id)) = scope.get_key_value(&target.into()) {
                    if !key.visibility.is_public() || !symbols[type_id].visibility().is_public() {
                        return Err(Error::TypeNotPublic {
                            location: use_path.location,
                            ty: type_id,
                        });
                    }
                    Ok(vec![(key.clone(), type_id)])
                } else {
                    return Err(Error::UnresolvedType {
                        location: use_path.location,
                        resolved_type: target,
                    });
                }
            }
            None => Ok(scope
                .iter()
                .filter_map(|(k, &id)| {
                    (k.visibility.is_public() && symbols[id].visibility().is_public())
                        .then_some((k.clone(), id))
                })
                .collect()),
        }
    }

    fn resolve_member_type(
        &mut self,
        path: &PathNode,
        root: TypeId,
        target: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        let scope_id = ctx.symbols[root].scope_id();
        if let Some(&type_id) = ctx.symbols[scope_id].types.get(&target.into()) {
            if !ctx.symbols[type_id].visibility().is_public() {
                return Err(Error::TypeNotPublic {
                    location: path.location,
                    ty: type_id,
                });
            }
            return Ok(type_id);
        }

        let method_type_id =
            self.resolve_method(root, target, ctx)
                .ok_or_else(|| Error::UnresolvedType {
                    location: path.location,
                    resolved_type: target,
                })?;
        if !ctx.symbols[method_type_id].visibility().is_public() {
            return Err(Error::MemberNotPublic {
                location: path.location,
                ty: root,
                field: target,
            });
        }
        return Ok(method_type_id);
    }

    fn resolve_module_type(
        &mut self,
        path: &PathNode,
        module: ModuleId,
        ty: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        let scope_id = ctx.symbols[module].scope_id;
        let type_id = ctx.symbols[scope_id]
            .types
            .get(&ty.into())
            .ok_or_else(|| Error::UnresolvedType {
                location: path.location,
                resolved_type: ty,
            })?
            .clone();
        if !ctx.symbols[type_id].visibility().is_public() {
            return Err(Error::TypeNotPublic {
                location: path.location,
                ty: type_id,
            });
        }
        return Ok(type_id);
    }
}
