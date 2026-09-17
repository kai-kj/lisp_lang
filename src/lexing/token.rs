use crate::prelude::*;

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

impl<'source> SToken<'source> {
    pub fn to_owned_token(&self) -> SOwnedToken {
        match self.value {
            Token::End => OwnedToken::End,
            Token::ParenLeft => OwnedToken::ParenLeft,
            Token::ParenRight => OwnedToken::ParenRight,
            Token::Quote => OwnedToken::Quote,
            Token::Symbol(value) => OwnedToken::Symbol(value.to_string()),
            Token::Integer(value) => OwnedToken::Integer(value),
            Token::Float(value) => OwnedToken::Float(value),
            Token::String(value) => OwnedToken::String(value.to_string()),
        }
        .span(self.span)
    }
}

pub type SOwnedToken = Spanned<OwnedToken>;

#[derive(Debug, Clone, PartialEq)]
pub enum OwnedToken {
    End,
    ParenLeft,
    ParenRight,
    Quote,
    Symbol(String),
    Integer(i64),
    Float(f64),
    String(String),
}

#[macro_export]
macro_rules! make_token {
    ($start:expr, $end:expr, $kind:ident $(($value:expr))? ) => {
        Ok(Token::$kind $(($value))?.span_between($start, $end))
    };
}
pub use make_token;

#[macro_export]
macro_rules! token_list {
    ($($kind:ident $(($value:expr))?),* $(,)?) => { Ok(vec![$(token_list!(@token $kind $(($value))?)),*]) };
    (@token String($value:expr)) => { OwnedToken::String(($value).into()).span_none() };
    (@token Symbol($value:expr)) => { OwnedToken::Symbol(($value).into()).span_none() };
    (@token $kind:ident $(($value:expr))?) => { OwnedToken::$kind $(($value))?.span_none() };
}
pub use token_list;
