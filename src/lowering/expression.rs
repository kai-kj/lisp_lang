use crate::{
    display::{WithDisplayContext, WithDisplayContextExt},
    runtime::value::Value,
    span::Spanned,
    symbol::SymbolId,
};

pub type SExpression = Spanned<Expression>;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Value),
    Variable(SymbolId),
    Call {
        call: Box<SExpression>,
        args: Vec<SExpression>,
    },
    If {
        cond: Box<SExpression>,
        t_branch: Box<SExpression>,
        f_branch: Box<SExpression>,
    },
    Function {
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
            (Expression::Literal(v), _) => write!(f, "Literal({})", v),
            (Expression::Variable(v), _) => write!(f, "Variable({})", v.context(self).no_indent()),
            (Expression::Call { call, args }, None) => {
                let args = args.iter().map(|c| format!("{}", c.context(self))).collect::<Vec<_>>();
                write!(f, "Call({}, {})", call.as_ref().context(self), args.join(", "))
            }
            (Expression::Call { call, args }, Some(indent_size)) => {
                writeln!(f, "Call(")?;
                writeln!(f, "{},", call.as_ref().context(self).indent())?;
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
            (Expression::Function { args, body }, None) => {
                let args = args.iter().map(|c| format!("{}", c.context(self))).collect::<Vec<_>>();
                let body = body.iter().map(|c| format!("{}", c.context(self))).collect::<Vec<_>>();
                write!(f, "Function({}, {})", args.join(", "), body.join(", "))
            }
            (Expression::Function { args, body }, Some(indent_size)) => {
                let args = args.iter().map(|c| format!("{}", c.context(self))).collect::<Vec<_>>();
                writeln!(f, "Function(")?;
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
