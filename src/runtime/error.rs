use crate::{
    frontend::parser::{ParserError, SParserError},
    span::{Spanned, SpannedExt},
};

pub type SRuntimeError = Spanned<RuntimeError>;

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    ParserError(Box<ParserError>),
    UnexpectedParamCount,
    UnexpectedParamType,
    VariableAlreadyDefined,
    VariableNotDefined,
    NotAFunction,
    UnexpectedMacro,
}

impl From<SParserError> for SRuntimeError {
    fn from(error: SParserError) -> Self {
        RuntimeError::ParserError(Box::new(error.value)).scopy(error.span)
    }
}
