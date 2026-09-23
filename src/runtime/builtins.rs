use {
    crate::runtime::{
        error::RuntimeError,
        value::{BuiltinFn, Value},
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

pub fn make_builtins<'s>() -> Vec<(&'static str, BuiltinFn)> {
    vec![
        (
            "and",
            BuiltinFn::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                if !a.is_truthy() { Ok(b.clone()) } else { Ok(a.clone()) }
            }),
        ),
        (
            "or",
            BuiltinFn::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                if a.is_truthy() { Ok(a.clone()) } else { Ok(b.clone()) }
            }),
        ),
        (
            "not",
            BuiltinFn::new(&["a"], |_, args| {
                get_args!(args; a);
                if a.is_truthy() { Ok(false.into()) } else { Ok(true.into()) }
            }),
        ),
        (
            "+",
            BuiltinFn::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, +)
            }),
        ),
        (
            "-",
            BuiltinFn::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, -)
            }),
        ),
        (
            "*",
            BuiltinFn::new(&["a", "b"], |session, args| {
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
            BuiltinFn::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, /)
            }),
        ),
        (
            "=",
            BuiltinFn::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                match (a, b) {
                    (Value::Null, Value::Null) => Ok(Value::Boolean(true)),
                    (Value::Boolean(a), Value::Boolean(b)) => Ok((a == b).into()),
                    (Value::Integer(a), Value::Integer(b)) => Ok((a == b).into()),
                    (Value::Float(a), Value::Float(b)) => Ok((a == b).into()),
                    (Value::String(a), Value::String(b)) => Ok((a == b).into()),
                    (Value::BuiltinFn(a), Value::BuiltinFn(b)) => Ok((a == b).into()),
                    (Value::UserFn(a), Value::UserFn(b)) => Ok((a == b).into()),
                    _ => Ok(false.into()),
                }
            }),
        ),
        (
            "<",
            BuiltinFn::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, <)
            }),
        ),
        (
            ">",
            BuiltinFn::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, >)
            }),
        ),
        (
            "<=",
            BuiltinFn::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, <=)
            }),
        ),
        (
            ">=",
            BuiltinFn::new(&["a", "b"], |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, >=)
            }),
        ),
        (
            "print",
            BuiltinFn::new(&["value"], |session, args| {
                get_args!(args; value);
                print!("{}", value.to_string(session));
                Ok(Value::Null)
            }),
        ),
        (
            "cons",
            BuiltinFn::new(&["head", "tail"], |_, args| {
                get_args!(args; head, tail);
                let mut values = vec![head.clone()];
                match tail {
                    Value::Null => {}
                    Value::List(tail) => values.extend(tail.iter().cloned()),
                    _ => return Err(RuntimeError::UnexpectedParamType),
                }
                Ok(Value::List(Rc::new(values)))
            }),
        ),
    ]
}
