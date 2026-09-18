use crate::span::Spanned;

pub type SToken<'source> = Spanned<Token<'source>>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token<'source> {
    End,
    ParenLeft,
    ParenRight,
    Quote,
    Symbol(&'source str),
    Integer(i64),
    Float(f64),
    String(&'source str),
}
