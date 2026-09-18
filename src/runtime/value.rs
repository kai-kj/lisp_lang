use crate::{lowering::expression::SExpression, runtime::error::RuntimeError, symbol::SymbolId};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    BuiltinFunction(BuiltinFunction),
    UserFunction(UserFunction),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Boolean(v) => *v,
            Value::Integer(v) => *v != 0,
            Value::Float(v) => *v != 0.0,
            Value::String(v) => !v.is_empty(),
            Value::BuiltinFunction(_) => true,
            Value::UserFunction(_) => true,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "Null"),
            Value::Boolean(v) => write!(f, "Boolean({})", v),
            Value::Integer(v) => write!(f, "Integer({})", v),
            Value::Float(v) => write!(f, "Float({})", v),
            Value::String(v) => write!(f, "String(\"{}\")", v),
            Value::BuiltinFunction(_) => write!(f, "BuiltinFunction"),
            Value::UserFunction(_) => write!(f, "UserFunction"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BuiltinFunction {
    pub args: usize,
    pub body: fn(&[Value]) -> Result<Value, RuntimeError>,
}

impl PartialEq for BuiltinFunction {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserFunction {
    pub args: Vec<SymbolId>,
    pub body: Vec<SExpression>,
}
