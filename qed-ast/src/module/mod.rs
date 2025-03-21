use qed_common::{define_arena_id, FileId};

use crate::{DefId, IdentId, NodeInfo, NodeType, Span, Visibility};

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
        is_self_prelude: bool,
        is_self_primitive: bool,
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
                //todo: for comment
                ModuleItemNode::Comment(_) => {}
            }
        }
        let module = Self {
            name,
            file_id,
            modules: {
                if !is_std {
                    modules.insert(0, (IdentId::STD, Visibility::Private, Default::default()));
                }
                modules
            },
            inline_modules,
            definitions: {
                if !is_std {
                    definitions.insert(0, DefId::USE_STD_PRELUDE);
                }
                definitions
            },
            visibility,
            is_std,
            is_self_std,
            is_self_prelude,
            is_self_primitive,
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
    Comment(String), // store comments
}
