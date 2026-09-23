use crate::span::Spanned;

pub type SEvent = Spanned<Event>;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    ListStart,
    ListEnd,
    Quote,
    Symbol(String), // TODO: str?
    Integer(i64),
    Float(f64),
    String(String), // TODO: str?
    SourceEnd,
}

pub trait EventEmitter {
    fn peek(&self) -> Result<&SEvent, SEventError>;
    fn next(&mut self) -> Result<SEvent, SEventError>;
}

pub type SEventError = Spanned<EventError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventError {
    UnexpectedEscapeSequence,
    UnterminatedString,
    UnexpectedValue,
}
