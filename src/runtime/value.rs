use crate::{
    expression::{ExpressionId, StringId},
    runtime::{
        error::RuntimeError,
        session::{BuiltinFunctionId, EnvironmentId, ParamRange, Session, UserFunctionId},
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(StringId),
    BuiltinFunction(BuiltinFunctionId),
    UserFunction(UserFunctionId),
}

impl Value {
    pub fn is_truthy(&self, session: &Session) -> bool {
        match self {
            Value::Null => false,
            Value::Boolean(v) => *v,
            Value::Integer(v) => *v != 0,
            Value::Float(v) => *v != 0.0,
            Value::String(v) => !session.get_string(*v).is_empty(),
            Value::BuiltinFunction(_) => true,
            Value::UserFunction(_) => true,
        }
    }

    pub fn to_string(&self, session: &Session) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Boolean(v) => format!("{}", v),
            Value::Integer(v) => format!("{}", v),
            Value::Float(v) => format!("{}", v),
            Value::String(v) => session.get_string(*v).to_string(),
            Value::BuiltinFunction(v) => format!("{}", v),
            Value::UserFunction(v) => format!("{}", v),
        }
    }

    pub fn to_string_debug(&self, session: &Session) -> String {
        match self {
            Value::Null => "Null".to_string(),
            Value::Boolean(v) => format!("Boolean({})", v),
            Value::Integer(v) => format!("Integer({})", v),
            Value::Float(v) => format!("Float({})", v),
            Value::String(v) => format!("String(\"{}\")", session.get_string(*v)),
            Value::BuiltinFunction(v) => format!("{}", v),
            Value::UserFunction(v) => format!("{}", v),
        }
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Boolean(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Integer(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UserFunction {
    pub params: ParamRange,
    pub body: ExpressionId,
    pub env: EnvironmentId,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuiltinFunction {
    pub params: &'static [&'static str],
    pub body: fn(&mut Session, &[Value]) -> Result<Value, RuntimeError>,
}

impl BuiltinFunction {
    pub fn new(
        params: &'static [&'static str],
        body: fn(&mut Session, &[Value]) -> Result<Value, RuntimeError>,
    ) -> Self {
        Self { params, body }
    }
}
