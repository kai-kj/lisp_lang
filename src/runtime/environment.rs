use {
    crate::{
        expression::SymbolId,
        runtime::{
            error::RuntimeError,
            session::{EnvironmentId, Session},
            value::Value,
        },
    },
    std::collections::HashMap,
};

pub struct Environment {
    parent: Option<EnvironmentId>,
    binds: HashMap<SymbolId, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Self { parent: None, binds: HashMap::new() }
    }

    pub fn child(parent: EnvironmentId) -> Self {
        Self { parent: Some(parent), binds: HashMap::new() }
    }

    pub fn def(
        session: &mut Session,
        environment: EnvironmentId,
        name: SymbolId,
        value: Value,
    ) -> Result<(), RuntimeError> {
        let environment = session.get_environment_mut(environment);
        environment.binds.insert(name, value);
        Ok(())
    }

    pub fn set(
        session: &mut Session,
        mut environment_id: EnvironmentId,
        name: SymbolId,
        value: Value,
    ) -> Result<(), RuntimeError> {
        loop {
            let environment = session.get_environment_mut(environment_id);

            if environment.binds.contains_key(&name) {
                environment.binds.insert(name, value);
                return Ok(());
            } else if let Some(parent_id) = &environment.parent {
                environment_id = *parent_id;
            } else {
                break;
            }
        }

        Err(RuntimeError::VariableNotDefined)
    }

    pub fn get(
        session: &Session,
        mut environment_id: EnvironmentId,
        name: SymbolId,
    ) -> Result<Value, RuntimeError> {
        loop {
            let environment = session.get_environment(environment_id);

            if let Some(v) = environment.binds.get(&name) {
                return Ok(*v);
            } else if let Some(parent_id) = &environment.parent {
                environment_id = *parent_id;
            } else {
                break;
            }
        }

        Err(RuntimeError::VariableNotDefined)
    }
}
