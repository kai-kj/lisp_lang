use {
    crate::{
        expression::{ExpressionId, ExpressionRange, SExpression, StringId, SymbolId, SymbolTable},
        runtime::{
            error::RuntimeError,
            value::{BuiltinFunction, UserFunction, Value},
        },
        util::{Arena, ArenaId},
    },
    std::{cell::RefCell, collections::HashMap, rc::Rc},
};

pub struct Session {
    expressions: Arena<SExpression>,
    symbols: SymbolTable,
    strings: Arena<String>,
    params: Vec<SymbolId>,
    builtin_functions: Vec<BuiltinFunction>,
    user_functions: Vec<UserFunction>,
    environments: Vec<Rc<Environment>>,
}

impl Session {
    pub fn new() -> Self {
        Session {
            expressions: Arena::new(),
            symbols: SymbolTable::new(),
            strings: Arena::new(),
            params: Vec::new(),
            builtin_functions: Vec::new(),
            user_functions: Vec::new(),
            environments: Vec::new(),
        }
    }

    pub fn get_expression(&self, id: ExpressionId) -> SExpression {
        *self.expressions.get(id)
    }

    pub fn get_expressions(&self, range: ExpressionRange) -> &[ArenaId<SExpression>] {
        self.expressions.get_children(range)
    }

    pub fn push_expression(&mut self, expression: SExpression) -> ExpressionId {
        self.expressions.push(expression)
    }

    pub fn push_expressions(&mut self, expressions: &[ExpressionId]) -> ExpressionRange {
        self.expressions.push_children(expressions)
    }

    pub fn get_symbol(&self, id: SymbolId) -> &str {
        self.symbols.get(id)
    }

    pub fn push_symbol(&mut self, symbol: &str) -> SymbolId {
        self.symbols.push(symbol)
    }

    pub fn get_string(&self, id: StringId) -> &str {
        self.strings.get(id)
    }

    pub fn push_string(&mut self, string: String) -> StringId {
        self.strings.push(string)
    }

    pub fn get_n_params(&self) -> usize {
        self.params.len()
    }

    pub fn get_params(&self, range: ParamRange) -> &[SymbolId] {
        &self.params[range.0..range.1]
    }

    pub fn push_param(&mut self, param: SymbolId) {
        self.params.push(param);
    }

    pub fn get_builtin_function(&self, id: BuiltinFunctionId) -> BuiltinFunction {
        self.builtin_functions[id.0]
    }

    pub fn push_builtin_function(
        &mut self,
        builtin_function: BuiltinFunction,
    ) -> BuiltinFunctionId {
        self.builtin_functions.push(builtin_function);
        BuiltinFunctionId(self.builtin_functions.len() - 1)
    }

    pub fn get_user_function(&self, id: UserFunctionId) -> UserFunction {
        self.user_functions[id.0]
    }

    pub fn push_user_function(&mut self, builtin_function: UserFunction) -> UserFunctionId {
        self.user_functions.push(builtin_function);
        UserFunctionId(self.user_functions.len() - 1)
    }

    pub fn root_environment(&mut self) -> Rc<Environment> {
        let env = Environment::new();
        self.environments.push(env.clone());
        env
    }

    pub fn child_environment(&mut self, parent: &Rc<Environment>) -> Rc<Environment> {
        let env = parent.child();
        self.environments.push(env.clone());
        env
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamRange(usize, usize);

impl ParamRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self(start, end)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuiltinFunctionId(usize);

impl std::fmt::Display for BuiltinFunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BuiltinFunction({:06x})", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UserFunctionId(usize);

impl std::fmt::Display for UserFunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UserFunction({:06x})", self.0)
    }
}

pub struct Environment {
    parent: Option<Rc<Environment>>,
    binds: RefCell<HashMap<SymbolId, Value>>,
}

impl Environment {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { parent: None, binds: RefCell::new(HashMap::new()) })
    }

    pub fn child(self: &Rc<Self>) -> Rc<Self> {
        Rc::new(Self { parent: Some(self.clone()), binds: RefCell::new(HashMap::new()) })
    }

    pub fn set(self: &Rc<Self>, name: SymbolId, value: Value) -> Result<(), RuntimeError> {
        // if self.binds.borrow().contains_key(&name) {
        //     Err(RuntimeError::VariableAlreadyDefined)
        // } else {
        //     self.binds.borrow_mut().insert(name, value);
        //     Ok(())
        // }
        self.binds.borrow_mut().insert(name, value);
        Ok(())
    }

    pub fn get(self: &Rc<Self>, name: SymbolId) -> Result<Value, RuntimeError> {
        if let Some(v) = self.binds.borrow().get(&name) {
            Ok(*v)
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            Err(RuntimeError::VariableNotDefined)
        }
    }
}
