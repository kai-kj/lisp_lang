use std::marker::PhantomData;

macro_rules! impl_copy {
    ($name:ident<$($T:ident),+>) => {
        impl<$($T),+> Copy for $name<$($T),+> {}
        impl<$($T),+> Clone for $name<$($T),+> {
            fn clone(&self) -> Self {
                *self
            }
        }
    };
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arena<V: Clone> {
    values: Vec<V>,
    children: Vec<ArenaId<V>>,
}

impl<V: Clone> Arena<V> {
    pub fn new() -> Self {
        Self { values: Vec::new(), children: Vec::new() }
    }

    pub fn get(&self, id: ArenaId<V>) -> &V {
        &self.values[id.0 as usize]
    }

    pub fn get_children(&self, range: ArenaRange<V>) -> &[ArenaId<V>] {
        let start = range.0 as usize;
        &self.children[start..start + range.1 as usize]
    }

    pub fn push(&mut self, value: V) -> ArenaId<V> {
        self.values.push(value);
        ArenaId::new(self.values.len() as u32 - 1)
    }

    pub fn push_children(&mut self, children: &[ArenaId<V>]) -> ArenaRange<V> {
        let range = ArenaRange::new(self.children.len() as u32, children.len() as u32);
        self.children.extend_from_slice(children);
        range
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ArenaRange<V>(u32, u32, PhantomData<V>);
impl_copy!(ArenaRange<V>);

impl<V> ArenaRange<V> {
    pub fn new(start: u32, len: u32) -> Self {
        Self(start, len, PhantomData)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ArenaId<V>(u32, PhantomData<V>);
impl_copy!(ArenaId<V>);

impl<V> ArenaId<V> {
    pub fn new(id: u32) -> Self {
        Self(id, PhantomData)
    }
}

#[macro_export]
macro_rules! check {
    ($condition:expr, $error:expr) => {
        if !$condition {
            return $error;
        }
    };
}
pub use check;
