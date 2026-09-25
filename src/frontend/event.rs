use crate::span::Spanned;

pub type SEvent = Spanned<Event>;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    ListStart,
    ListEnd,
    Quote,
    Symbol(String),
    Integer(i64),
    Float(f64),
    String(String), // TODO: str?
    SourceEnd,
}

pub trait Reader {
    fn peek(&self) -> Result<&SEvent, SReadError>;
    fn next(&mut self) -> Result<SEvent, SReadError>;
}

pub type SReadError = Spanned<ReadError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReadError {
    InvalidEscapeSequence,
    UnterminatedString,
    UnsupportedSyntaxValue,
    InvalidSymbol,
}
