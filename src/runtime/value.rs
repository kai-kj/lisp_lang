use {
    crate::{
        expression::{ExpressionId, SymbolId},
        runtime::{
            error::RuntimeError,
            session::{ParamRange, Session},
        },
        util::Environment,
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
    BuiltinFn(Rc<BuiltinFn>),
    UserFn(Rc<UserFn>),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Symbol(_) => true,
            Value::List(v) => !v.is_empty(),
            Value::Boolean(v) => *v,
            Value::Integer(v) => *v != 0,
            Value::Float(v) => *v != 0.0,
            Value::String(v) => !v.is_empty(),
            Value::BuiltinFn(_) => true,
            Value::UserFn(_) => true,
        }
    }

    pub fn format(&self, sesh: &Session) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Symbol(v) => sesh.get_symbol(*v).to_string(),
            Value::List(v) => {
                let items = v.iter().map(|v| v.format(sesh)).collect::<Vec<_>>();
                format!("({})", items.join(", "))
            }
            Value::Boolean(v) => format!("{}", v),
            Value::Integer(v) => format!("{}", v),
            Value::Float(v) => format!("{}", v),
            Value::String(v) => format!("{}", v),
            Value::BuiltinFn(v) => Self::format_builtin_fn(v),
            Value::UserFn(v) => Self::format_user_fn(sesh, v),
        }
    }

    pub fn format_debug(&self, sesh: &Session) -> String {
        match self {
            Value::Null => "Null".to_string(),
            Value::Symbol(v) => sesh.get_symbol(*v).to_string(),
            Value::List(v) => {
                let items = v.iter().map(|v| v.format(sesh)).collect::<Vec<_>>();
                format!("List({})", items.join(", "))
            }
            Value::Boolean(v) => format!("Boolean({})", v),
            Value::Integer(v) => format!("Integer({})", v),
            Value::Float(v) => format!("Float({})", v),
            Value::String(v) => format!("String(\"{:?}\")", v),
            Value::BuiltinFn(v) => Self::format_builtin_fn(v),
            Value::UserFn(v) => Self::format_user_fn(sesh, v),
        }
    }

    fn format_builtin_fn(v: &BuiltinFn) -> String {
        let mut p = v.req_params.iter().map(|p| p.to_string()).collect::<Vec<_>>();
        if v.rest_param {
            p.push("...".to_string())
        }
        format!("BuiltinFn({})", p.join(", "))
    }

    fn format_user_fn(sesh: &Session, v: &UserFn) -> String {
        let p = sesh.get_params(v.req_params);
        let mut p = p.iter().map(|p| sesh.get_symbol(*p).to_string()).collect::<Vec<_>>();
        if v.rest_param.is_some() {
            p.push("...".to_string())
        }
        format!("UserFn({})", p.join(", "))
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
pub struct UserFn {
    pub req_params: ParamRange,
    pub rest_param: Option<SymbolId>,
    pub body: ExpressionId,
    pub env: Rc<Environment<SymbolId, Value>>,
}

impl UserFn {
    pub fn accepts(&self, n_args: usize) -> bool {
        n_args >= self.req_params.len()
            && (self.rest_param.is_some() || n_args == self.req_params.len())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuiltinFn {
    pub req_params: &'static [&'static str],
    pub rest_param: bool,
    pub body: fn(&mut Session, &[Value]) -> Result<Value, RuntimeError>,
}

impl BuiltinFn {
    pub fn new(
        req_params: &'static [&'static str],
        rest_param: bool,
        body: fn(&mut Session, &[Value]) -> Result<Value, RuntimeError>,
    ) -> Self {
        Self { req_params, rest_param, body }
    }

    pub fn accepts(&self, n_args: usize) -> bool {
        n_args >= self.req_params.len() && (self.rest_param || n_args == self.req_params.len())
    }
}
