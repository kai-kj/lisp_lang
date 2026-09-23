use {
    crate::{
        expression::SymbolId,
        runtime::{error::RuntimeError, value::Value},
    },
    std::{cell::RefCell, collections::HashMap, rc::Rc},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    parent: Option<Rc<Environment>>,
    binds: RefCell<HashMap<SymbolId, Value>>,
}

impl Environment {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { parent: None, binds: RefCell::new(HashMap::new()) })
    }

    pub fn child(self: &Rc<Environment>) -> Rc<Self> {
        Rc::new(Self { parent: Some(self.clone()), binds: RefCell::new(HashMap::new()) })
    }

    pub fn def(self: &Rc<Environment>, name: SymbolId, value: Value) -> Result<(), RuntimeError> {
        self.binds.borrow_mut().insert(name, value);
        Ok(())
    }

    pub fn set(self: &Rc<Environment>, name: SymbolId, value: Value) -> Result<(), RuntimeError> {
        if self.binds.borrow().contains_key(&name) {
            self.binds.borrow_mut().insert(name, value);
            return Ok(());
        }

        if let Some(parent) = &self.parent {
            return parent.set(name, value);
        }

        Err(RuntimeError::VariableNotDefined)
    }

    pub fn get(self: &Rc<Environment>, name: SymbolId) -> Result<Value, RuntimeError> {
        if let Some(v) = self.binds.borrow().get(&name) {
            return Ok(v.clone());
        }

        if let Some(parent) = &self.parent {
            return parent.get(name);
        }

        Err(RuntimeError::VariableNotDefined)
    }
}
