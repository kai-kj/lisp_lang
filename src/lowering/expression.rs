use crate::{
    display::{WithDisplayContext, WithDisplayContextExt},
    runtime::value::Value,
    span::Spanned,
};

pub type SExpression<'s> = Spanned<Expression<'s>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression<'s> {
    Literal(Value<'s>),
    Variable(&'s str),
    Call {
        call: Box<SExpression<'s>>,
        args: Vec<SExpression<'s>>,
    },
    If {
        cond: Box<SExpression<'s>>,
        t_branch: Box<SExpression<'s>>,
        f_branch: Box<SExpression<'s>>,
    },
    Function {
        args: Vec<&'s str>,
        body: Vec<SExpression<'s>>,
    },
    Define {
        name: &'s str,
        value: Box<SExpression<'s>>,
    },
}

impl<'v, 's> std::fmt::Display for WithDisplayContext<'v, SExpression<'s>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.make_indent())?;
        match (&self.value.value, self.indent_size) {
            (Expression::Literal(v), _) => write!(f, "Literal({})", v),
            (Expression::Variable(v), _) => write!(f, "Variable(\"{}\")", v),
            (Expression::Call { call, args }, None) => {
                let args = args.iter().map(|c| format!("{}", c.disp())).collect::<Vec<_>>();
                write!(f, "Call({}, {})", call.as_ref().disp(), args.join(", "))
            }
            (Expression::Call { call, args }, Some(indent_size)) => {
                writeln!(f, "Call(")?;
                writeln!(f, "{},", call.as_ref().disp().indent())?;
                args.iter().try_for_each(|c| writeln!(f, "{},", c.disp().indent()))?;
                write!(f, "{})", " ".repeat(self.indent_level * indent_size))
            }
            (Expression::If { cond, t_branch, f_branch }, None) => {
                write!(
                    f,
                    "If({}, {}, {})",
                    cond.as_ref().disp(),
                    t_branch.as_ref().disp(),
                    f_branch.as_ref().disp()
                )
            }
            (Expression::If { cond, t_branch, f_branch }, Some(indent_size)) => {
                writeln!(f, "If(")?;
                writeln!(f, "{},", cond.as_ref().disp().indent())?;
                writeln!(f, "{},", t_branch.as_ref().disp().indent())?;
                writeln!(f, "{},", f_branch.as_ref().disp().indent())?;
                write!(f, "{})", " ".repeat(self.indent_level * indent_size))
            }
            (Expression::Function { args, body }, None) => {
                let args = args.iter().map(|c| format!("{}", c)).collect::<Vec<_>>();
                let body = body.iter().map(|c| format!("{}", c.disp())).collect::<Vec<_>>();
                write!(f, "Function({}, {})", args.join(", "), body.join(", "))
            }
            (Expression::Function { args, body }, Some(indent_size)) => {
                let args = args.iter().map(|c| format!("{}", c)).collect::<Vec<_>>();
                writeln!(f, "Function(")?;
                writeln!(
                    f,
                    "{}({}),",
                    " ".repeat((self.indent_level + 1) * indent_size),
                    args.join(", ")
                )?;
                body.iter().try_for_each(|c| writeln!(f, "{},", c.disp().indent()))?;
                write!(f, "{})", " ".repeat(self.indent_level * indent_size))
            }
            (Expression::Define { name, value }, None) => {
                write!(f, "Define({}, {})", name, value.as_ref().disp())
            }
            (Expression::Define { name, value }, Some(indent_size)) => {
                writeln!(f, "Define(")?;
                writeln!(f, "{},", name)?;
                writeln!(f, "{}", value.as_ref().disp().indent())?;
                write!(f, "{})", " ".repeat(self.indent_level * indent_size))
            }
        }
    }
}
