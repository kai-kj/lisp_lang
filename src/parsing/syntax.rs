use crate::{
    display::{WithDisplayContext, WithDisplayContextExt},
    span::Spanned,
};

pub type SSyntax<'s> = Spanned<Syntax<'s>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Syntax<'s> {
    Symbol(&'s str),
    Integer(i64),
    Float(f64),
    String(String),
    List(Vec<SSyntax<'s>>),
}

impl<'v, 's> std::fmt::Display for WithDisplayContext<'v, SSyntax<'s>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.make_indent())?;
        match &self.value.value {
            Syntax::Symbol(v) => write!(f, "Symbol(\"{}\")", v),
            Syntax::Integer(v) => write!(f, "Integer({})", v),
            Syntax::Float(v) => write!(f, "Float({})", v),
            Syntax::String(v) => write!(f, "String(\"{}\")", v),
            Syntax::List(v) => {
                if self.indent_size.is_some() {
                    write!(f, "List(\n")?;
                    v.iter().try_for_each(|c| writeln!(f, "{},", c.disp().indent()))?;
                    write!(f, "{})", self.make_indent())
                } else {
                    let v = v.iter().map(|c| format!("{}", c.disp())).collect::<Vec<_>>();
                    write!(f, "List({})", v.join(", "))
                }
            }
        }
    }
}
