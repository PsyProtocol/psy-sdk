use std::collections::{HashMap, HashSet};

use qed_ast::{
    IdentId, Identifier, Location, ModuleId, PathNode, TraitImplNode, UncheckedType, UseNode,
    VisitorContext,
};
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{
    AstVisualizer, CheckedPathNode, Error, Implementer, Inferer, Result, TypeChecker,
    TypeCheckerVisitorContext, TypeId, TypeKey,
};

#[derive(Debug)]
pub struct ResolverCtxt {}

impl ResolverCtxt {
    pub fn new() -> Self {
        Self {}
    }
}

pub trait Resolver<F: Clone + From<u32> + ContextFelt, C> {
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

    fn resolve_module(
        &self,
        module: &Identifier,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<ModuleId>;

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
    fn resolve_path(
        &mut self,
        path: &PathNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedPathNode> {
        let current_module_id = ctx.symbols.current_module_id().unwrap();

        let mut src_module = match path.root.as_ref() {
            Some(ty) => match ty {
                UncheckedType::Basic(name) if let Ok(module) = self.resolve_module(name, ctx) => {
                    module
                }
                _ => {
                    if !path.segments.is_empty() {
                        return Err(Error::InvalidPathSegment {
                            location: path.location,
                            segment: path.segments[0].id,
                        });
                    }
                    let root_type_id = self.typecheck(ty, ctx)?;
                    let type_id =
                        self.resolve_member_type(path, root_type_id, path.target.id, ctx)?;
                    return Ok(CheckedPathNode::new(
                        None,
                        self.substitute_all(type_id, ctx)?,
                        path.location,
                    ));
                }
            },
            None => {
                if !path.segments.is_empty() {
                    return Err(Error::InvalidPathSegment {
                        location: path.location,
                        segment: path.segments[0].id,
                    });
                }
                if let Some(var_id) = ctx.symbols.get_variable(None, &path.target) {
                    return Ok(CheckedPathNode::new(
                        Some(var_id),
                        self.substitute_all(ctx.symbols[var_id].ty, ctx)?,
                        path.location,
                    ));
                }
                let type_id = ctx.symbols.get_type_id(None, path.target).ok_or_else(|| {
                    Error::UnresolvedType {
                        location: path.location,
                        resolved_type: path.target.id,
                    }
                })?;
                return Ok(CheckedPathNode::new(
                    None,
                    self.substitute_all(type_id, ctx)?,
                    path.location,
                ));
            }
        };

        for (i, segment) in path.segments.iter().enumerate() {
            if let Some(&target_module_id) = ctx.symbols[src_module]
                .children
                .iter()
                .find(|&&id| ctx.symbols[id].name == segment.id)
            {
                if !ctx.symbols[target_module_id].visibility.is_public() {
                    return Err(Error::ModuleNotPublic {
                        location: path.location,
                        module: ctx.symbols[target_module_id].name.id,
                    });
                }
                src_module = target_module_id;
            } else {
                if i != path.segments.len() - 1 {
                    return Err(Error::InvalidPathSegment {
                        location: path.location,
                        segment: segment.id,
                    });
                }
                let root_type_id = self.resolve_module_type(path, src_module, segment.id, ctx)?;
                let type_id = self.resolve_member_type(path, root_type_id, path.target.id, ctx)?;
                return Ok(CheckedPathNode::new(
                    None,
                    self.substitute_all(type_id, ctx)?,
                    path.location,
                ));
            }
        }

        let type_id = self.resolve_module_type(path, src_module, path.target.id, ctx)?;
        return Ok(CheckedPathNode::new(None, type_id, path.location));
    }

    fn resolve_use(
        &self,
        use_path: &UseNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<Vec<(TypeKey, TypeId)>> {
        let mut current_module_id = self.resolve_module(&use_path.kind, ctx)?;
        ctx.add_module_reference(current_module_id, use_path.kind.location, false);

        let traverse_path_segment = |current: ModuleId, segment: &Identifier| {
            if let Some(module) = ctx.symbols[current]
                .children
                .iter()
                .find(|&&id| ctx.symbols[id].name == segment.id)
                .copied()
            {
                if ctx.symbols[module].visibility.is_public() {
                    ctx.add_module_reference(module, segment.location, false);
                    Ok(module)
                } else {
                    Err(Error::ModuleNotPublic {
                        location: use_path.location,
                        module: segment.id,
                    })
                }
            } else {
                Err(Error::ModuleNotFound {
                    location: use_path.location,
                    module: segment.id,
                })
            }
        };

        current_module_id = use_path
            .segments
            .iter()
            .try_fold(current_module_id, traverse_path_segment)?;

        let scope_id = ctx.symbols[current_module_id].scope_id;

        match use_path.target {
            Some(target)
                if let Some((key, &type_id)) =
                    ctx.symbols[scope_id].types.get_key_value(&target.id.into()) =>
            {
                if !key.visibility.is_public() || !ctx.symbols[type_id].visibility().is_public() {
                    return Err(Error::TypeNotPublic {
                        location: use_path.location,
                        ty: type_id,
                    });
                }
                Ok(vec![(key.clone(), type_id)])
            }
            Some(target) => Err(Error::UnresolvedType {
                location: use_path.location,
                resolved_type: target.id,
            }),
            None => Ok(ctx.symbols[scope_id]
                .types
                .iter()
                .filter_map(|(k, &id)| {
                    (k.visibility.is_public() && ctx.symbols[id].visibility().is_public())
                        .then_some((k.clone(), id))
                })
                .collect()),
        }
    }

    fn resolve_module(
        &self,
        module: &Identifier,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<ModuleId> {
        let current_module_id = ctx.symbols.current_module_id().unwrap();
        Ok(match module.id {
            IdentId::SELF => current_module_id,
            IdentId::CRATE => {
                let mut root_id = current_module_id;
                while let Some(parent) = ctx.symbols[root_id].parent {
                    root_id = parent;
                }
                root_id
            }
            IdentId::SUPER => {
                ctx.symbols[current_module_id]
                    .parent
                    .ok_or(Error::NoParentModule {
                        location: module.location,
                    })?
            }
            name => ctx
                .symbols
                .modules()
                .iter()
                .position(|x| x.name == name)
                .map(ModuleId)
                .filter(|&id| ctx.symbols[current_module_id].children.contains(&id))
                .ok_or(Error::ModuleNotFound {
                    location: module.location,
                    module: name,
                })?,
        })
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

        let method_type_id = self.find_method(root, target, ctx)?;
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
