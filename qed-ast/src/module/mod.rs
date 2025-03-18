use qed_common::{define_arena_id, Arena, FileId};

use crate::{DefId, DefinitionNode, IdentId, NodeInfo, NodeType, Span, Visibility};

define_arena_id!(ModuleId);

impl ModuleId {
    pub const fn root() -> Self {
        Self(0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleKind {
    File { file_id: FileId },
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseNode {
    pub visibility: Visibility,
    pub kind: IdentId,
    pub segments: Vec<IdentId>,
    pub target: Option<IdentId>,
    pub span: Span,
}

impl NodeInfo for UseNode {
    fn node_type(&self) -> NodeType {
        NodeType::UseDef
    }
}

#[derive(Clone, Debug)]
pub struct ModuleNode {
    pub name: IdentId,
    pub file_id: FileId,
    pub modules: Vec<(IdentId, Visibility, Span)>,
    pub inline_modules: Vec<ModuleNode>,
    pub definitions: Vec<DefId>,
    pub visibility: Visibility,

    pub is_std: bool,
    pub is_self_std: bool,
    pub is_self_prelude: bool,
    pub is_self_primitive: bool,

    pub span: Span,
}

impl ModuleNode {
    pub fn new(
        name: IdentId,
        file_id: FileId,
        visibility: Visibility,
        module_items: Vec<ModuleItemNode>,
        is_std: bool,
        is_self_std: bool,
        def_nodes: &mut Arena<DefId, DefinitionNode>,
        span: Span,
    ) -> Self {
        let mut inline_modules = vec![];
        let mut modules = vec![];
        let mut definitions = vec![];
        for item in module_items.into_iter() {
            match item {
                ModuleItemNode::InlineModule(m) => inline_modules.push(m),
                ModuleItemNode::ModuleDecl(m) => modules.push(m),
                ModuleItemNode::Definition(d) => definitions.push(d),
            }
        }
        let module = Self {
            name,
            file_id,
            modules: {
                if !is_std {
                    modules.insert(
                        0,
                        (IdentId::STD, Visibility::Private, Span::new(file_id, 0, 0)),
                    );
                }
                modules
            },
            inline_modules,
            definitions: {
                if !is_std {
                    let def_id = def_nodes.alloc_item(DefinitionNode::Use(UseNode {
                        visibility: Visibility::Private,
                        kind: IdentId::STD,
                        segments: vec![IdentId::PRELUDE],
                        target: None,
                        span: Span::new(file_id, 0, 0),
                    }));

                    definitions.insert(0, def_id);
                }
                definitions
            },
            visibility,
            is_std,
            is_self_std,
            is_self_prelude: name == IdentId::PRELUDE,
            is_self_primitive: name == IdentId::PRIMITIVE,
            span,
        };
        module
    }
}

#[derive(Clone, Debug)]
pub enum ModuleItemNode {
    ModuleDecl((IdentId, Visibility, Span)),
    InlineModule(ModuleNode),
    Definition(DefId),
}
