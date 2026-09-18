use crate::{
    lowering::lowerer::{LowererError, SLowererError},
    parsing::parser::SParserError,
    span::Spanned,
};

pub type SRuntimeError = Spanned<RuntimeError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuntimeError {
    LowererError(LowererError),
    VariableAlreadyDefined,
    VariableNotDefined,
    NotAFunction,
    InvalidArgCount,
    InvalidArgType,
}

impl From<SLowererError> for SRuntimeError {
    fn from(value: SLowererError) -> Self {
        SRuntimeError { value: RuntimeError::LowererError(value.value), span: value.span }
    }
}

impl From<SParserError> for SRuntimeError {
    fn from(value: SParserError) -> Self {
        SLowererError::from(value).into()
    }
}
