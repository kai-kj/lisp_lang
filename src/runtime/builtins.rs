use crate::runtime::{
    error::RuntimeError,
    value::{BuiltinFn, Value},
};

macro_rules! get_args {
    ($args:expr; $($name:ident),* $(,)?) => {
        let mut args_iter = ($args).iter();
        $(let $name = args_iter.next().ok_or(RuntimeError::UnexpectedParamCount)?;)*
    };
}

macro_rules! numerical_bin_op {
    ($a:expr, $b:expr, $op:tt) => {
        match ($a, $b) {
            (Value::Integer(a), Value::Integer(b)) => Ok((*a $op *b).into()),
            (Value::Integer(a), Value::Float(b)) => Ok(((*a as f64) $op *b).into()),
            (Value::Float(a), Value::Integer(b)) => Ok((*a $op (*b as f64)).into()),
            (Value::Float(a), Value::Float(b)) => Ok((*a $op *b).into()),
            _ => Err(RuntimeError::UnexpectedParamType),
        }
    };
}

pub fn make_builtins<'s>() -> Vec<(&'static str, BuiltinFn)> {
    vec![
        (
            "and",
            BuiltinFn::new(&["a", "b"], false, |_, args| {
                get_args!(args; a, b);
                if !a.is_truthy() { Ok(b.clone()) } else { Ok(a.clone()) }
            }),
        ),
        (
            "or",
            BuiltinFn::new(&["a", "b"], false, |_, args| {
                get_args!(args; a, b);
                if a.is_truthy() { Ok(a.clone()) } else { Ok(b.clone()) }
            }),
        ),
        (
            "not",
            BuiltinFn::new(&["a"], false, |_, args| {
                get_args!(args; a);
                if a.is_truthy() { Ok(false.into()) } else { Ok(true.into()) }
            }),
        ),
        (
            "+",
            BuiltinFn::new(&[], true, |_, args| {
                args.iter().try_fold(0.into(), |sum, value| numerical_bin_op!(&sum, value, +))
            }),
        ),
        (
            "-",
            BuiltinFn::new(&[], true, |_, args| {
                args.iter().try_fold(0.into(), |sum, value| numerical_bin_op!(&sum, value, -))
            }),
        ),
        (
            "*",
            BuiltinFn::new(&[], true, |_, args| {
                args.iter().try_fold(1.into(), |sum, value| numerical_bin_op!(&sum, value, *))
            }),
        ),
        (
            "/",
            BuiltinFn::new(&[], true, |_, args| {
                args.iter().try_fold(1.into(), |sum, value| numerical_bin_op!(&sum, value, /))
            }),
        ),
        (
            "=",
            BuiltinFn::new(&["a", "b"], false, |_, args| {
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
            BuiltinFn::new(&["a", "b"], false, |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, <)
            }),
        ),
        (
            ">",
            BuiltinFn::new(&["a", "b"], false, |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, >)
            }),
        ),
        (
            "<=",
            BuiltinFn::new(&["a", "b"], false, |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, <=)
            }),
        ),
        (
            ">=",
            BuiltinFn::new(&["a", "b"], false, |_, args| {
                get_args!(args; a, b);
                numerical_bin_op!(a, b, >=)
            }),
        ),
        (
            "print",
            BuiltinFn::new(&[], true, |sesh, args| {
                args.iter().try_for_each(|value| {
                    print!("{}", value.format(sesh));
                    Ok(())
                })?;
                Ok(Value::Null)
            }),
        ),
        (
            "cons",
            BuiltinFn::new(&["head", "tail"], false, |_, args| {
                get_args!(args; head, tail);
                let mut values = vec![head.clone()];
                match tail {
                    Value::Null => {}
                    Value::List(tail) => values.extend(tail.iter().cloned()),
                    _ => return Err(RuntimeError::UnexpectedParamType),
                }
                Ok(Value::list(values))
            }),
        ),
        (
            "list",
            BuiltinFn::new(&[], true, |_, args| {
                let mut values = vec![];
                args.iter().try_for_each(|value| {
                    values.push(value.clone());
                    Ok(())
                })?;
                Ok(Value::list(values))
            }),
        ),
    ]
}
