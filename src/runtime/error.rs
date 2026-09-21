use crate::{
    frontend::parser::{ParserError, SParserError},
    span::{Spanned, SpannedExt},
};

pub type SRuntimeError = Spanned<RuntimeError>;

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    ParserError(ParserError),
    UnexpectedParamCount,
    UnexpectedParamType,
    VariableAlreadyDefined,
    VariableNotDefined,
    NotAFunction,
}

impl From<SParserError> for SRuntimeError {
    fn from(error: SParserError) -> Self {
        RuntimeError::ParserError(error.value).sinherit(&error)
    }
}
