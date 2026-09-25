use {
    crate::{
        check,
        runtime::{
            error::RuntimeError,
            value::{BuiltinFn, Value},
        },
    },
    std::{
        ops::{Add, Div, Mul, Sub},
        rc::Rc,
    },
};

macro_rules! get_args {
    ($args:expr; $($name:ident),* $(,)?) => {
        let mut args_iter = ($args).iter();
        $(let $name = args_iter.next().ok_or(RuntimeError::InvalidArgCount)?;)*
    };
}

macro_rules! arithmetic_op {
    ($a:expr, $b:expr, $fn_int:expr, $fn_float:expr) => {
        match ($a, $b) {
            (Value::Integer(a), Value::Integer(b)) => {
                $fn_int(*a, *b).map(Value::Integer).ok_or(RuntimeError::IntegerOverflow)
            }
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float($fn_float(*a as f64, *b))),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float($fn_float(*a, *b as f64))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float($fn_float(*a, *b))),
            _ => Err(RuntimeError::InvalidArgType),
        }
    };
}

macro_rules! comparison_op {
    ($a:expr, $b:expr, $op:tt) => {
        match ($a, $b) {
            (Value::Integer(a), Value::Integer(b)) => Ok((*a $op *b).into()),
            (Value::Integer(a), Value::Float(b)) => Ok(((*a as f64) $op *b).into()),
            (Value::Float(a), Value::Integer(b)) => Ok((*a $op (*b as f64)).into()),
            (Value::Float(a), Value::Float(b)) => Ok((*a $op *b).into()),
            _ => Err(RuntimeError::InvalidArgType),
        }
    };
}

pub fn make_builtins<'s>() -> Vec<(&'static str, BuiltinFn)> {
    vec![
        (
            "cons",
            BuiltinFn::new(&["head", "tail"], false, |_, args| {
                get_args!(args; head, tail);
                let mut values = vec![head.clone()];
                match tail {
                    Value::Null => {}
                    Value::List(tail) => values.extend(tail.iter().cloned()),
                    _ => return Err(RuntimeError::InvalidArgType),
                }
                Ok(Value::List(Rc::new(values)))
            }),
        ),
        (
            "head",
            BuiltinFn::new(&["list"], false, |_, args| {
                get_args!(args; list);
                match list {
                    Value::List(values) => values.first().cloned().ok_or(RuntimeError::EmptyList),
                    _ => Err(RuntimeError::InvalidArgType),
                }
            }),
        ),
        (
            "tail",
            BuiltinFn::new(&["list"], false, |_, args| {
                get_args!(args; list);
                match list {
                    Value::List(values) => {
                        let (_, tail) = values.split_first().ok_or(RuntimeError::EmptyList)?;
                        Ok(Value::List(Rc::new(tail.to_vec())))
                    }
                    _ => Err(RuntimeError::InvalidArgType),
                }
            }),
        ),
        (
            "type-of",
            BuiltinFn::new(&["value"], false, |sesh, args| {
                get_args!(args; value);
                let name = match value {
                    Value::Null => "null",
                    Value::Symbol(_) => "symbol",
                    Value::List(_) => "list",
                    Value::Boolean(_) => "boolean",
                    Value::Integer(_) => "integer",
                    Value::Float(_) => "float",
                    Value::String(_) => "string",
                    Value::BuiltinFn(_) | Value::UserFn(_) => "function",
                };
                Ok(Value::Symbol(sesh.push_symbol(name)))
            }),
        ),
        (
            "list", // TODO: implement as macro
            BuiltinFn::new(&[], true, |_, args| {
                let mut values = vec![];
                args.iter().try_for_each(|value| {
                    values.push(value.clone());
                    Ok(())
                })?;
                Ok(Value::List(Rc::new(values)))
            }),
        ),
        (
            "=",
            BuiltinFn::new(&["a", "b"], false, |_, args| {
                get_args!(args; a, b);
                match (a, b) {
                    (Value::Null, Value::Null) => Ok(Value::Boolean(true)),
                    (Value::Symbol(a), Value::Symbol(b)) => Ok((a == b).into()),
                    (Value::List(a), Value::List(b)) => Ok((a == b).into()),
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
            "+",
            BuiltinFn::new(&[], true, |_, args| {
                args.iter().try_fold(Value::Integer(0), |acc, value| {
                    arithmetic_op!(&acc, value, i64::checked_add, f64::add)
                })
            }),
        ),
        (
            "-",
            BuiltinFn::new(&[], true, |_, args| {
                let (first, rest) = args.split_first().ok_or(RuntimeError::InvalidArgCount)?;
                if rest.is_empty() {
                    return arithmetic_op!(&Value::Integer(0), first, i64::checked_sub, f64::sub);
                }
                rest.iter().try_fold(first.clone(), |acc, value| {
                    arithmetic_op!(&acc, value, i64::checked_sub, f64::sub)
                })
            }),
        ),
        (
            "*",
            BuiltinFn::new(&[], true, |_, args| {
                args.iter().try_fold(Value::Integer(0), |acc, value| {
                    arithmetic_op!(&acc, value, i64::checked_mul, f64::mul)
                })
            }),
        ),
        (
            "/",
            BuiltinFn::new(&[], true, |_, args| {
                let (first, rest) = args.split_first().ok_or(RuntimeError::InvalidArgCount)?;
                if rest.is_empty() {
                    check!(first != &Value::Integer(0), Err(RuntimeError::DivisionByZero));
                    return arithmetic_op!(&Value::Integer(0), first, i64::checked_div, f64::div);
                }
                rest.iter().try_fold(first.clone(), |acc, value| {
                    check!(value != &Value::Integer(0), Err(RuntimeError::DivisionByZero));
                    arithmetic_op!(&acc, value, i64::checked_div, f64::div)
                })
            }),
        ),
        (
            "<",
            BuiltinFn::new(&["a", "b"], false, |_, args| {
                get_args!(args; a, b);
                comparison_op!(a, b, <)
            }),
        ),
        (
            ">",
            BuiltinFn::new(&["a", "b"], false, |_, args| {
                get_args!(args; a, b);
                comparison_op!(a, b, >)
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
            "error",
            BuiltinFn::new(&["message"], false, |_, args| {
                get_args!(args; message);
                match message {
                    Value::String(message) => Err(RuntimeError::User(message.as_ref().clone())),
                    _ => Err(RuntimeError::InvalidArgType),
                }
            }),
        ),
    ]
}
