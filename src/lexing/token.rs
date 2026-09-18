use crate::span::Spanned;

pub type SToken<'s> = Spanned<Token<'s>>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token<'s> {
    End,
    ParenLeft,
    ParenRight,
    Quote,
    Symbol(&'s str),
    Integer(i64),
    Float(f64),
    String(&'s str),
}
