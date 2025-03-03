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

macro_rules! ident_ids {
    //deal with the first element, the starting index is 0
    ($first:ident => $str_val:expr $(, $name:ident => $str_vals:expr)*) => {
        impl IdentId {
            pub const $first: IdentId = IdentId(0);
            ident_ids!(@internal 1, $($name => $str_vals),*);
        }

        pub const IDENT_MAPPING: &[(IdentId, &str)] = &[
            (IdentId::$first, $str_val),
            $( (IdentId::$name, $str_vals) ),*
        ];
    };

    // deal with the subsequent elements, the index increases
    (@internal $index:expr, $first:ident => $str_val:expr $(, $name:ident => $str_vals:expr)*) => {
        pub const $first: IdentId = IdentId($index);
        ident_ids!(@internal $index + 1, $($name => $str_vals),*);
    };


    // recursion termination condition (no more elements)
    (@internal $index:expr,) => {};
}
//
ident_ids!(
    TYPE_UNKNOWN => "unknown", // 0
    TYPE_BOOL => "bool",
    TYPE_FELT => "Felt",
    TYPE_VOID => "void",
    TYPE_ARRAY => "[]",
    TYPE_TUPLE => "Tuple",
    TYPE_HASH => "Hash",
    TYPE_SELF => "Self",

    SELF => "self", //8
    SUPER => "super",
    CRATE => "crate",

    STD => "std", // 11
    PRELUDE => "prelude",
    PRIMITIVE => "primitive",

    DERIVE => "derive", // 14
    NEW => "new",
    TEST => "test",

    TYPE_U32 => "u32"
);

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

    pub fn intern_lambda(&mut self) -> IdentId {
        let s: Ident = "__LAMBDA_FUNCTON__".into();
        IdentId({
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ident_ids_mapping() {
        //mapping of expected (IdentId, &str)
        let expected_mapping = vec![
            (IdentId::TYPE_UNKNOWN, "unknown"),
            (IdentId::TYPE_BOOL, "bool"),
            (IdentId::TYPE_FELT, "Felt"),
            (IdentId::TYPE_VOID, "void"),
            (IdentId::TYPE_ARRAY, "[]"),
            (IdentId::TYPE_TUPLE, "Tuple"),
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
            (IdentId::TYPE_U32, "u32"),
        ];

        // check if the length of the mapping is correct
        assert_eq!(IDENT_MAPPING.len(), expected_mapping.len());

        //iterate through each (IdentId, &str) and check if it matches
        for (i, (ident, expected_str)) in expected_mapping.iter().enumerate() {
            let (actual_ident, actual_str) = IDENT_MAPPING[i];
            assert_eq!(actual_ident, *ident, "IdentId mismatch at index {}", i);
            assert_eq!(actual_str, *expected_str, "String mismatch at index {}", i);
        }
    }
}
