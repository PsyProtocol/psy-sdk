use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    ops::{Index, IndexMut},
};

use qed_common::{define_arena_id, Arena};
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident(pub SmolStr);

impl Display for Ident {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Ident {
    fn from(s: &str) -> Self {
        if s.len() <= 22 {
            Ident(SmolStr::new_inline(s))
        } else {
            Ident(SmolStr::new(s))
        }
    }
}

impl From<String> for Ident {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

define_arena_id!(IdentId);

impl IdentId {
    pub const TYPE_UNKNOWN: IdentId = IdentId(0);
    pub const TYPE_BOOL: IdentId = IdentId(1);
    pub const TYPE_FELT: IdentId = IdentId(2);
    pub const TYPE_VOID: IdentId = IdentId(3);
    pub const TYPE_ARRAY: IdentId = IdentId(4);
    pub const TYPE_HASH: IdentId = IdentId(5);
    pub const TYPE_SELF: IdentId = IdentId(6);

    pub const SELF: IdentId = IdentId(7);
    pub const SUPER: IdentId = IdentId(8);
    pub const CRATE: IdentId = IdentId(9);

    pub const STD: IdentId = IdentId(10);
    pub const PRELUDE: IdentId = IdentId(11);
    pub const PRIMITIVE: IdentId = IdentId(12);

    pub const DERIVE: IdentId = IdentId(13);
    pub const NEW: IdentId = IdentId(14);
    pub const TEST: IdentId = IdentId(15);

    pub const FN_SIG: IdentId = IdentId(16);
}

pub const IDENT_MAPPING: &[(IdentId, &str)] = &[
    (IdentId::TYPE_UNKNOWN, "unknown"),
    (IdentId::TYPE_BOOL, "bool"),
    (IdentId::TYPE_FELT, "Felt"),
    (IdentId::TYPE_VOID, "void"),
    (IdentId::TYPE_ARRAY, "[]"),
    (IdentId::TYPE_HASH, "Hash"),
    (IdentId::TYPE_SELF, "Self"),
    (IdentId::SELF, "self"),
    (IdentId::SUPER, "super"),
    (IdentId::CRATE, "crate"),
    (IdentId::STD, "std"),
    (IdentId::PRELUDE, "prelude"),
    (IdentId::PRIMITIVE, "primitive"),
    (IdentId::DERIVE, "derive"),
    (IdentId::NEW, "new"),
    (IdentId::TEST, "test"),
    (IdentId::FN_SIG, "fn"),
];

#[derive(Clone, Debug, Default)]
pub struct Interner {
    pool: Arena<IdentId, Ident>,
    index: HashMap<Ident, usize>,
}

impl Interner {
    pub fn new() -> Self {
        let mut interner = Self {
            pool: Arena::new(),
            index: HashMap::default(),
        };

        interner.intern_idents(IDENT_MAPPING.iter().map(|(_, name)| *name));

        interner
    }

    pub fn intern_ident<S: Into<Ident>>(&mut self, s: S) -> IdentId {
        let s = s.into();
        IdentId(if let Some(&idx) = self.index.get(&s) {
            idx
        } else {
            let idx = self.pool.len();
            self.pool.alloc_item(s.clone());
            self.index.insert(s.clone(), idx);
            idx
        })
    }

    pub fn intern_idents<S: Into<Ident>>(
        &mut self,
        s: impl IntoIterator<Item = S>,
    ) -> Vec<IdentId> {
        let mut result = Vec::new();
        for s in s {
            result.push(self.intern_ident(s.into()));
        }
        result
    }
}

impl Index<IdentId> for Interner {
    type Output = Ident;
    fn index(&self, index: IdentId) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<IdentId> for Interner {
    fn index_mut(&mut self, index: IdentId) -> &mut Self::Output {
        &mut self.pool[index]
    }
}
