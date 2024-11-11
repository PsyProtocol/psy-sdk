use std::ops::{Index, IndexMut};

#[derive(Clone, Debug)]
pub struct Arena<I: From<usize> + Into<usize>, T> {
    pub items: Vec<T>,
    _marker: std::marker::PhantomData<I>,
}

impl<I, T> Arena<I, T>
where
    I: From<usize> + Into<usize>,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn alloc_item(&mut self, item: T) -> I {
        self.items.push(item);
        I::from(self.items.len() - 1)
    }

    pub fn alloc_items(&mut self, items: impl IntoIterator<Item = T>) -> Vec<I> {
        let mut result = Vec::new();
        for item in items {
            result.push(self.alloc_item(item));
        }
        result
    }
}

impl<I, T> Default for Arena<I, T>
where
    I: From<usize> + Into<usize>,
{
    fn default() -> Self {
        Self {
            items: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<I, T> Index<I> for Arena<I, T>
where
    I: From<usize> + Into<usize>,
{
    type Output = T;
    fn index(&self, index: I) -> &Self::Output {
        &self.items[index.into()]
    }
}
impl<I, T> IndexMut<I> for Arena<I, T>
where
    I: From<usize> + Into<usize>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.items[index.into()]
    }
}

#[macro_export]
macro_rules! define_arena_id {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name(pub usize);

        impl From<usize> for $name {
            fn from(i: usize) -> Self {
                $name(i)
            }
        }

        impl From<$name> for usize {
            fn from(i: $name) -> Self {
                i.0
            }
        }
    };
}
