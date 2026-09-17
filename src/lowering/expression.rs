use crate::prelude::*;

pub type SExpression = Spanned<Expression>;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Value),
    Variable(SymbolId),
    Call {
        name: SymbolId,
        args: Vec<SExpression>,
    },
    If {
        cond: Box<SExpression>,
        t_branch: Box<SExpression>,
        f_branch: Box<SExpression>,
    },
    Lambda {
        args: Vec<SymbolId>,
        body: Vec<SExpression>,
    },
    Define {
        name: SymbolId,
        value: Box<SExpression>,
    },
}

impl<'a> std::fmt::Display for WithDisplayContext<'a, SExpression> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.make_indent())?;
        match (&self.value.value, self.indent_size) {
            (Expression::Literal(v), _) => write!(f, "Literal({})", v.context(self).no_indent()),
            (Expression::Variable(v), _) => write!(f, "Variable({})", v.context(self).no_indent()),
            (Expression::Call { name, args }, None) => {
                let args = args.iter().map(|c| format!("{}", c.context(self))).collect::<Vec<_>>();
                write!(f, "Call({}, {})", name.context(self), args.join(", "))
            }
            (Expression::Call { name, args }, Some(indent_size)) => {
                writeln!(f, "Call(")?;
                writeln!(f, "{},", name.context(self).indent())?;
                args.iter().try_for_each(|c| writeln!(f, "{},", c.context(self).indent()))?;
                write!(f, "{})", " ".repeat(self.indent * indent_size))
            }
            (Expression::If { cond, t_branch, f_branch }, None) => {
                write!(
                    f,
                    "If({}, {}, {})",
                    cond.as_ref().context(self),
                    t_branch.as_ref().context(self),
                    f_branch.as_ref().context(self)
                )
            }
            (Expression::If { cond, t_branch, f_branch }, Some(indent_size)) => {
                writeln!(f, "If(")?;
                writeln!(f, "{},", cond.as_ref().context(self).indent())?;
                writeln!(f, "{},", t_branch.as_ref().context(self).indent())?;
                writeln!(f, "{},", f_branch.as_ref().context(self).indent())?;
                write!(f, "{})", " ".repeat(self.indent * indent_size))
            }
            (Expression::Lambda { args, body }, None) => {
                let args = args.iter().map(|c| format!("{}", c.context(self))).collect::<Vec<_>>();
                let body = body.iter().map(|c| format!("{}", c.context(self))).collect::<Vec<_>>();
                write!(f, "Lambda({}, {})", args.join(", "), body.join(", "))
            }
            (Expression::Lambda { args, body }, Some(indent_size)) => {
                let args = args.iter().map(|c| format!("{}", c.context(self))).collect::<Vec<_>>();
                writeln!(f, "Lambda(")?;
                writeln!(
                    f,
                    "{}({}),",
                    " ".repeat((self.indent + 1) * indent_size),
                    args.join(", ")
                )?;
                body.iter().try_for_each(|c| writeln!(f, "{},", c.context(self).indent()))?;
                write!(f, "{})", " ".repeat(self.indent * indent_size))
            }
            (Expression::Define { name, value }, None) => {
                write!(f, "Define({}, {})", name.context(self), value.as_ref().context(self))
            }
            (Expression::Define { name, value }, Some(indent_size)) => {
                writeln!(f, "Define(")?;
                writeln!(f, "{},", name.context(self).indent())?;
                writeln!(f, "{}", value.as_ref().context(self).indent())?;
                write!(f, "{})", " ".repeat(self.indent * indent_size))
            }
        }
    }
}

#[macro_export]
macro_rules! expression_list {
    ($($kind:ident $args:tt),* $(,)?) => {{
        let mut _symbol_table = SymbolTable::new();
        Ok(expression_list!(_symbol_table; $($kind $args),*))
    }};

    ($table:ident; $($kind:ident $args:tt),* $(,)?) => {{
        let _symbol_table = &mut $table;
        vec![$(expression_list!(@expr _symbol_table; $kind $args)),*]
    }};

    (@expr $table:ident; Literal(Null)) => { Expression::Literal(Value::Null).span_none() };

    (@expr $table:ident; Literal(Symbol($value:expr))) => {
        Expression::Literal(Value::Symbol($table.add_symbol($value))).span_none()
    };

    (@expr $table:ident; Literal($kind:ident($value:expr))) => {
        Expression::Literal(Value::$kind($value).into()).span_none()
    };

    (@expr $table:ident; Variable($name:expr)) => {
        Expression::Variable($table.add_symbol($name)).span_none()
    };

    (@expr $table:ident; Call($name:expr; $($kind:ident $args:tt),* $(,)?)) => {
        Expression::Call {
            name: $table.add_symbol($name),
            args: vec![$(expression_list!(@expr $table; $kind $args)),*],
        }.span_none()
    };

    (@expr $table:ident; If(
        $cond_kind:ident $cond_args:tt;
        $t_kind:ident $t_args:tt;
        $f_kind:ident $f_args:tt
    )) => {
        Expression::If {
            cond: Box::new(expression_list!(@expr $table; $cond_kind $cond_args)),
            t_branch: Box::new(expression_list!(@expr $table; $t_kind $t_args)),
            f_branch: Box::new(expression_list!(@expr $table; $f_kind $f_args)),
        }.span_none()
    };

    (@expr $table:ident; Lambda(
        ($($arg:expr),* $(,)?);
        $($kind:ident $args:tt),* $(,)?
    )) => {
        Expression::Lambda {
            args: vec![
                $($table.add_symbol($arg)),*
            ],
            body: vec![
                $(expression_list!(@expr $table; $kind $args)),*
            ],
        }.span_none()
    };

    // ----- Define -----

    (@expr $table:ident; Define(
        $name:expr;
        $kind:ident $args:tt
    )) => {
        Expression::Define {
            name: $table.add_symbol($name),
            value: Box::new(
                expression_list!(@expr $table; $kind $args)
            ),
        }.span_none()
    };
}

pub use expression_list;
