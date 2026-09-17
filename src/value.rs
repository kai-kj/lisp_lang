use crate::prelude::WithDisplayContext;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

impl std::fmt::Display for WithDisplayContext<'_, Value> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value {
            Value::Null => write!(f, "Null"),
            Value::Boolean(v) => write!(f, "Boolean({})", v),
            Value::Integer(v) => write!(f, "Integer({})", v),
            Value::Float(v) => write!(f, "Float({})", v),
            Value::String(v) => write!(f, "String(\"{}\")", v),
        }
    }
}
