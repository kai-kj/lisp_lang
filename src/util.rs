use std::{cell::RefCell, collections::HashMap, hash::Hash, marker::PhantomData, rc::Rc};

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

#[derive(Debug, Clone, PartialEq)]
pub struct Environment<K: Eq + Hash, V: Clone> {
    parent: Option<Rc<Environment<K, V>>>,
    binds: RefCell<HashMap<K, V>>,
}

impl<K: Eq + Hash, V: Clone> Environment<K, V> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { parent: None, binds: RefCell::new(HashMap::new()) })
    }

    pub fn child(self: &Rc<Environment<K, V>>) -> Rc<Self> {
        Rc::new(Self { parent: Some(self.clone()), binds: RefCell::new(HashMap::new()) })
    }

    pub fn def(self: &Rc<Environment<K, V>>, name: K, value: V) -> Result<(), RuntimeError> {
        if self.binds.borrow().contains_key(&name) {
            return Err(RuntimeError::AlreadyDefinedVariable);
        }

        self.binds.borrow_mut().insert(name, value);
        Ok(())
    }

    pub fn set(self: &Rc<Environment<K, V>>, name: K, value: V) -> Result<(), RuntimeError> {
        if self.binds.borrow().contains_key(&name) {
            self.binds.borrow_mut().insert(name, value);
            return Ok(());
        }

        if let Some(parent) = &self.parent {
            return parent.set(name, value);
        }

        Err(RuntimeError::UndefinedVariable)
    }

    pub fn get(self: &Rc<Environment<K, V>>, name: K) -> Result<V, RuntimeError> {
        if let Some(v) = self.binds.borrow().get(&name) {
            return Ok(v.clone());
        }

        if let Some(parent) = &self.parent {
            return parent.get(name);
        }

        Err(RuntimeError::UndefinedVariable)
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
use crate::runtime::error::RuntimeError;
pub use check;
