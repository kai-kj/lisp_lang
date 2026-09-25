use crate::{
    frontend::parser::{ParseError, SParseError},
    span::{Spanned, SpannedExt},
};

pub type SRuntimeError = Spanned<RuntimeError>;

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    Parse(Box<ParseError>),
    User(String),
    InvalidArgCount,
    InvalidArgType,
    AlreadyDefinedVariable,
    UndefinedVariable,
    NotCallable,
    UnexpectedMacro,
    IntegerOverflow,
    DivisionByZero,
    EmptyList,
}

impl From<SParseError> for SRuntimeError {
    fn from(error: SParseError) -> Self {
        RuntimeError::Parse(Box::new(error.value)).scopy(error.span)
    }
}
