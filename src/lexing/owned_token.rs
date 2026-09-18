use crate::{
    lexing::token::{SToken, Token},
    span::{Spanned, SpannedExt},
};

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
