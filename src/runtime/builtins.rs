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
                .ok_or(RuntimeError::InvalidArgType)?;
        )*
    };
}

pub fn make_builtins<'s>() -> Vec<(&'static str, BuiltinFunction<'s>)> {
    vec![
        (
            "+",
            BuiltinFunction {
                args: 2,
                body: |args| {
                    get_args!(args; a, b);
                    match (a, b) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                        _ => Err(RuntimeError::InvalidArgType),
                    }
                },
            },
        ),
        (
            "-",
            BuiltinFunction {
                args: 2,
                body: |args| {
                    get_args!(args; a, b);
                    match (a, b) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                        _ => Err(RuntimeError::InvalidArgType),
                    }
                },
            },
        ),
        (
            "*",
            BuiltinFunction {
                args: 2,
                body: |args| {
                    get_args!(args; a, b);
                    match (a, b) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                        (Value::String(a), Value::Integer(b)) => {
                            Ok(Value::String(a.repeat(*b as usize)))
                        }
                        _ => Err(RuntimeError::InvalidArgType),
                    }
                },
            },
        ),
        (
            "/",
            BuiltinFunction {
                args: 2,
                body: |args| {
                    get_args!(args; a, b);
                    match (a, b) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a / b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                        _ => Err(RuntimeError::InvalidArgType),
                    }
                },
            },
        ),
        (
            "print",
            BuiltinFunction {
                args: 1,
                body: |args| {
                    get_args!(args; a);
                    match a {
                        Value::Null => print!("null"),
                        Value::Boolean(v) => print!("{}", v),
                        Value::Integer(v) => print!("{}", v),
                        Value::Float(v) => print!("{}", v),
                        Value::String(v) => print!("{}", v),
                        Value::BuiltinFunction(_) => print!("builtin_function"),
                        Value::UserFunction(_) => print!("user_function"),
                    }
                    Ok(Value::Null)
                },
            },
        ),
    ]
}
