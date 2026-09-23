use {
    crate::{
        expression::{ExpressionId, SymbolId},
        runtime::{
            environment::Environment,
            error::RuntimeError,
            session::{ParamRange, Session},
        },
    },
    std::rc::Rc,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Symbol(SymbolId),
    List(Rc<Vec<Value>>),
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(Rc<String>),
    BuiltinFunction(Rc<BuiltinFunction>),
    UserFunction(Rc<UserFunction>),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Symbol(_) => true,
            Value::List(_) => true,
            Value::Boolean(v) => *v,
            Value::Integer(v) => *v != 0,
            Value::Float(v) => *v != 0.0,
            Value::String(v) => v.is_empty(),
            Value::BuiltinFunction(_) => true,
            Value::UserFunction(_) => true,
        }
    }

    pub fn to_string(&self, session: &Session) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Symbol(v) => session.get_symbol(*v).to_string(),
            Value::List(v) => {
                let items = v.iter().map(|v| v.to_string(session)).collect::<Vec<_>>();
                format!("({})", items.join(", "))
            }
            Value::Boolean(v) => format!("{}", v),
            Value::Integer(v) => format!("{}", v),
            Value::Float(v) => format!("{}", v),
            Value::String(v) => format!("{}", v),
            Value::BuiltinFunction(v) => {
                let p = v.params.iter().map(|p| p.to_string()).collect::<Vec<_>>();
                format!("BuiltinFunction({})", p.join(", "))
            }
            Value::UserFunction(v) => {
                let p = session.get_params(v.params);
                let p = p.iter().map(|p| session.get_symbol(*p).to_string()).collect::<Vec<_>>();
                format!("UserFunction({})", p.join(", "))
            }
        }
    }

    pub fn to_string_debug(&self, session: &Session) -> String {
        match self {
            Value::Null => "Null".to_string(),
            Value::Symbol(v) => session.get_symbol(*v).to_string(),
            Value::List(v) => {
                let items = v.iter().map(|v| v.to_string(session)).collect::<Vec<_>>();
                format!("List({})", items.join(", "))
            }
            Value::Boolean(v) => format!("Boolean({})", v),
            Value::Integer(v) => format!("Integer({})", v),
            Value::Float(v) => format!("Float({})", v),
            Value::String(v) => format!("String(\"{:?}\")", v),
            Value::BuiltinFunction(v) => {
                let p = v.params.iter().map(|p| p.to_string()).collect::<Vec<_>>();
                format!("BuiltinFunction({})", p.join(", "))
            }
            Value::UserFunction(v) => {
                let p = session.get_params(v.params);
                let p = p.iter().map(|p| session.get_symbol(*p).to_string()).collect::<Vec<_>>();
                format!("UserFunction({})", p.join(", "))
            }
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

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserFunction {
    pub params: ParamRange,
    pub body: ExpressionId,
    pub env: Rc<Environment>,
}

#[derive(Debug, Clone, PartialEq)]
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
