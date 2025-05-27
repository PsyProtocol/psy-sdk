use qed_common::{define_arena_id, Arena, FileId};

use crate::{
    Comment, DefId, DefinitionNode, IdentId, Identifier, Location, NodeInfo, NodeType, Visibility,
};

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
    pub kind: Identifier,
    pub segments: Vec<Identifier>,
    pub target: Option<Identifier>,
    pub comments: Vec<Comment>,
    pub location: Location,
}

impl NodeInfo for UseNode {
    fn node_type(&self) -> NodeType {
        NodeType::UseDef
    }
}

#[derive(Clone, Debug)]
pub struct ModuleNode {
    pub name: Identifier,
    pub file_id: FileId,
    pub modules: Vec<(Identifier, Visibility, Location)>,
    pub inline_modules: Vec<ModuleNode>,
    pub definitions: Vec<DefId>,
    pub visibility: Visibility,
    pub is_std: bool,
    pub comments: Vec<Comment>,
    pub location: Location,
}

impl ModuleNode {
    pub fn new(
        name: Identifier,
        file_id: FileId,
        visibility: Visibility,
        module_items: Vec<ModuleItemNode>,
        is_std: bool,
        def_nodes: &mut Arena<DefId, DefinitionNode>,
        comments: Vec<Comment>,
        location: Location,
    ) -> Self {
        let mut inline_modules = vec![];
        let mut modules = vec![];
        let mut definitions = vec![];
        for item in module_items.into_iter() {
            match item {
                ModuleItemNode::InlineModule(m) => inline_modules.push(m),
                ModuleItemNode::ModuleDecl(m) => modules.push(m),
                ModuleItemNode::Definition(d) => definitions.push(d),
                ModuleItemNode::Comment(_c) => todo!(),
            }
        }
        let module = Self {
            name,
            file_id,
            modules,
            inline_modules,
            definitions: {
                if !is_std {
                    let def_id = def_nodes.alloc_item(DefinitionNode::Use(UseNode {
                        visibility: Visibility::Private,
                        kind: Identifier::new(IdentId::STD, Location::new(file_id, 0, 0)),
                        segments: vec![Identifier::new(
                            IdentId::PRELUDE,
                            Location::new(file_id, 0, 0),
                        )],
                        target: None,
                        comments: vec![],
                        location: Location::new(file_id, 0, 0),
                    }));

                    definitions.insert(0, def_id);
                }
                definitions
            },
            visibility,
            is_std,
            comments,
            location,
        };
        module
    }

    pub fn is_std(&self) -> bool {
        self.is_std
    }

    pub fn is_self_std(&self) -> bool {
        self.name == IdentId::STD
    }

    pub fn is_self_prelude(&self) -> bool {
        self.name == IdentId::PRELUDE
    }

    pub fn is_self_primitive(&self) -> bool {
        self.name == IdentId::PRIMITIVE
    }
}

#[derive(Clone, Debug)]
pub enum ModuleItemNode {
    ModuleDecl((Identifier, Visibility, Location)),
    InlineModule(ModuleNode),
    Definition(DefId),
    Comment(Comment),
}
