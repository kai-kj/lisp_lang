use {
    crate::runtime::{
        error::RuntimeError,
        value::{BuiltinFunction, Value},
    },
    std::rc::Rc,
};

macro_rules! get_args {
    ($args:expr; $($name:ident),* $(,)?) => {
        let mut args_iter = ($args).iter();
        $(let $name = args_iter.next().ok_or(RuntimeError::UnexpectedParamCount)?;)*
    };
}

macro_rules! numerical_bin_op {
    ($a:expr, $b:expr, $op:tt $(, $pat:pat => $body:expr)* $(,)?) => {
        match ($a, $b) {
            (Value::Integer(a), Value::Integer(b)) => Ok((*a $op *b).into()),
            (Value::Integer(a), Value::Float(b)) => Ok(((*a as f64) $op *b).into()),
            (Value::Float(a), Value::Integer(b)) => Ok((*a $op (*b as f64)).into()),
            (Value::Float(a), Value::Float(b)) => Ok((*a $op *b).into()),
            $($pat => $body,)*
            _ => Err(RuntimeError::UnexpectedParamType),
        }
    };
}

pub fn make_builtins<'s>() -> Vec<(&'static str, BuiltinFunction)> {
    vec![
        (
            "and",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                if !a.is_truthy() { Ok(b.clone()) } else { Ok(a.clone()) }
            }),
        ),
        (
            "or",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                if a.is_truthy() { Ok(a.clone()) } else { Ok(b.clone()) }
            }),
        ),
        (
            "not",
            BuiltinFunction::new(&["a"], |_, args| {
                get_args!(args; a);
                if a.is_truthy() { Ok(false.into()) } else { Ok(true.into()) }
            }),
        ),
        (
            "+",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, +)
            }),
        ),
        (
            "-",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, -)
            }),
        ),
        (
            "*",
            BuiltinFunction::new(&["a", "b"], |session, args| {
                get_args!(args; a, b);
                numerical_bin_op!(
                    a, b, *,
                    (Value::String(a), Value::Integer(b)) => {
                        Ok(Value::String(Rc::new(a.repeat(*b as usize))))
                    },
                )
            }),
        ),
        (
            "/",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, /)
            }),
        ),
        (
            "=",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                match (a, b) {
                    (Value::Null, Value::Null) => Ok(Value::Boolean(true)),
                    (Value::Boolean(a), Value::Boolean(b)) => Ok((a == b).into()),
                    (Value::Integer(a), Value::Integer(b)) => Ok((a == b).into()),
                    (Value::Float(a), Value::Float(b)) => Ok((a == b).into()),
                    (Value::String(a), Value::String(b)) => Ok((a == b).into()),
                    (Value::BuiltinFunction(a), Value::BuiltinFunction(b)) => Ok((a == b).into()),
                    (Value::UserFunction(a), Value::UserFunction(b)) => Ok((a == b).into()),
                    _ => Ok(false.into()),
                }
            }),
        ),
        (
            "<",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, <)
            }),
        ),
        (
            ">",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, >)
            }),
        ),
        (
            "<=",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, <=)
            }),
        ),
        (
            ">=",
            BuiltinFunction::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, >=)
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
