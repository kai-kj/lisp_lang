use crate::runtime::{
    error::RuntimeError,
    value::{BuiltinFunction, Value},
};

macro_rules! get_args {
    ($args:expr; $($name:ident),* $(,)?) => {
        let mut args_iter = ($args).iter();
        $(
            let $name = args_iter
                .next()
                .ok_or(RuntimeError::UnexpectedParamCount)?;
        )*
    };
}

pub fn make_builtins<'s>() -> [(&'static str, BuiltinFunction); 5] {
    [
        (
            "+",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                    _ => Err(RuntimeError::UnexpectedParamType),
                }
            }),
        ),
        (
            "-",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                    _ => Err(RuntimeError::UnexpectedParamType),
                }
            }),
        ),
        (
            "*",
            BuiltinFunction::new(&["a", "b"], |session, args| {
                get_args!(args; a, b);
                match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                    (Value::String(a), Value::Integer(b)) => {
                        let a = session.get_string(*a);
                        Ok(Value::String(session.push_string(a.repeat(*b as usize))))
                    }
                    _ => Err(RuntimeError::UnexpectedParamType),
                }
            }),
        ),
        (
            "/",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a / b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                    _ => Err(RuntimeError::UnexpectedParamType),
                }
            }),
        ),
        (
            "print",
            BuiltinFunction::new(&["value"], |session, args| {
                get_args!(args; value);
                print!("{}", value.to_string(session));
                Ok(Value::Null)
            }),
        ),
    ]
}
