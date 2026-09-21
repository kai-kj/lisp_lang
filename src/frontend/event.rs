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
    String(String),
    SourceEnd,
}
