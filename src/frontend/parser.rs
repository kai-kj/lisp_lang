use {
    crate::{
        expression::{Expression, ExpressionId, ExpressionRange},
        frontend::{
            event::Event,
            reader::{Reader, ReaderError},
        },
        runtime::session::{ParamRange, Session},
        span::{Span, Spanned, SpannedExt},
    },
    std::cmp::PartialEq,
};

pub struct Parser {
    scratch: Vec<ExpressionId>,
}

macro_rules! error {
    ($kind:ident, $span:expr) => {
        Err(ParserError::$kind.scopy($span))
    };
}

impl Parser {
    pub fn new() -> Self {
        Self { scratch: Vec::new() }
    }

    pub fn parse(
        mut self,
        session: &mut Session,
        reader: &mut Reader<'_>,
    ) -> Result<ExpressionRange, SParserError> {
        while reader.peek()?.value != Event::SourceEnd {
            let root = self.parse_expression(session, reader)?;
            self.scratch.push(root);
        }
        Ok(self.push_scratch(session, 0))
    }

    fn parse_expression(
        &mut self,
        session: &mut Session,
        reader: &mut Reader<'_>,
    ) -> Result<ExpressionId, SParserError> {
        let event = reader.next()?;

        let result = match event.value {
            Event::ListStart => return self.parse_list(session, reader, event.span),
            Event::ListEnd => return error!(UnexpectedListEnd, event.span),
            Event::SourceEnd => return error!(UnexpectedSourceEnd, event.span),
            Event::Quote => return self.parse_expression_quoted(session, reader),
            Event::Symbol(name) if name == ReservedSymbols::NULL => Expression::Null,
            Event::Symbol(name) if name == ReservedSymbols::TRUE => Expression::Boolean(true),
            Event::Symbol(name) if name == ReservedSymbols::FALSE => Expression::Boolean(false),
            Event::Symbol(name) if ReservedSymbols::matches(&name) => {
                return error!(UnexpectedReservedSymbol, event.span);
            }
            Event::Symbol(name) => Expression::Symbol(session.push_symbol(&name)),
            Event::Integer(value) => Expression::Integer(value),
            Event::Float(value) => Expression::Float(value),
            Event::String(value) => Expression::String(session.push_string(&value)),
        };

        self.push_expression(session, result, event.span)
    }

    fn parse_expression_quoted(
        &mut self,
        session: &mut Session,
        reader: &mut Reader<'_>,
    ) -> Result<ExpressionId, SParserError> {
        let event = reader.next()?;

        let result = match event.value {
            Event::ListStart => return self.parse_list_quoted(session, reader, event.span),
            Event::ListEnd => return error!(UnexpectedListEnd, event.span),
            Event::Quote => Expression::Symbol(session.push_symbol("quote")),
            Event::Symbol(v) => Expression::Symbol(session.push_symbol(&v)),
            Event::Integer(v) => Expression::Integer(v),
            Event::Float(v) => Expression::Float(v),
            Event::String(v) => Expression::String(session.push_string(&v)),
            Event::SourceEnd => return error!(UnexpectedSourceEnd, event.span),
        };

        self.push_expression(session, result, event.span)
    }

    fn parse_list(
        &mut self,
        session: &mut Session,
        reader: &mut Reader<'_>,
        parent_span: Span,
    ) -> Result<ExpressionId, SParserError> {
        let result = match &reader.peek()?.value {
            Event::Symbol(v) if v == ReservedSymbols::DO => {
                self.parse_do(session, reader, parent_span)?
            }
            Event::Symbol(v) if v == ReservedSymbols::IF => {
                self.parse_if(session, reader, parent_span)?
            }
            Event::Symbol(v) if v == ReservedSymbols::FN => {
                self.parse_fn(session, reader, parent_span)?
            }
            Event::Symbol(v) if v == ReservedSymbols::DEF => {
                self.parse_def_or_set(session, reader, parent_span, true)?
            }
            Event::Symbol(v) if v == ReservedSymbols::SET => {
                self.parse_def_or_set(session, reader, parent_span, false)?
            }
            Event::Symbol(v) if v == ReservedSymbols::QUOTE => {
                self.parse_quote(session, reader, parent_span)?
            }
            Event::ListEnd => {
                reader.next()?; // consume list end
                self.push_expression(session, Expression::Null, parent_span)?
            }
            _ => self.parse_call(session, reader, parent_span)?,
        };

        Ok(result)
    }

    fn parse_list_quoted(
        &mut self,
        session: &mut Session,
        reader: &mut Reader<'_>,
        parent_span: Span,
    ) -> Result<ExpressionId, SParserError> {
        let scratch_start = self.scratch.len();

        while reader.peek()?.value != Event::ListEnd {
            let child = self.parse_expression_quoted(session, reader)?;
            self.scratch.push(child);
        }

        let body = self.push_scratch(session, scratch_start);

        Self::consume_list_end(reader)?;
        self.push_expression(session, Expression::List(body), parent_span)
    }

    fn parse_do(
        &mut self,
        session: &mut Session,
        reader: &mut Reader<'_>,
        parent_span: Span,
    ) -> Result<ExpressionId, SParserError> {
        reader.next()?; // consume "do"

        let scratch_start = self.scratch.len();
        while reader.peek()?.value != Event::ListEnd {
            let child = self.parse_expression(session, reader)?;
            self.scratch.push(child);
        }

        let body = self.push_scratch(session, scratch_start);

        Self::consume_list_end(reader)?;
        self.push_expression(session, Expression::Do(body), parent_span)
    }

    fn parse_call(
        &mut self,
        session: &mut Session,
        reader: &mut Reader<'_>,
        parent_span: Span,
    ) -> Result<ExpressionId, SParserError> {
        let call = self.parse_expression(session, reader)?;

        let scratch_start = self.scratch.len();
        while reader.peek()?.value != Event::ListEnd {
            let arg = self.parse_expression(session, reader)?;
            self.scratch.push(arg);
        }

        let args = self.push_scratch(session, scratch_start);

        Self::consume_list_end(reader)?;
        self.push_expression(session, Expression::Call { call, args }, parent_span)
    }

    fn parse_if(
        &mut self,
        session: &mut Session,
        reader: &mut Reader<'_>,
        parent_span: Span,
    ) -> Result<ExpressionId, SParserError> {
        reader.next()?; // consume "if"

        let cond = self.parse_expression(session, reader)?;
        let t_branch = self.parse_expression(session, reader)?;
        let f_branch = self.parse_expression(session, reader)?;

        Self::consume_list_end(reader)?;
        self.push_expression(session, Expression::If { cond, t_branch, f_branch }, parent_span)
    }

    fn parse_fn(
        &mut self,
        session: &mut Session,
        reader: &mut Reader<'_>,
        parent_span: Span,
    ) -> Result<ExpressionId, SParserError> {
        reader.next()?; // consume "fn"

        let list_start = reader.next()?;
        if list_start.value != Event::ListStart {
            return error!(ExpectedFunctionParams, list_start.span);
        }

        // make sure no recursive parsing happens before ParamRange::new()
        let params_start = session.get_n_params();
        loop {
            let event = reader.next()?;
            match event.value {
                Event::ListEnd => break,
                Event::Symbol(name) if ReservedSymbols::matches(&name) => {
                    return error!(UnexpectedReservedSymbol, event.span);
                }
                Event::Symbol(name) => {
                    let param = session.push_symbol(&name);
                    session.push_param(param);
                }
                _ => return error!(ExpectedSymbol, event.span),
            }
        }
        let param_range = ParamRange::new(params_start, session.get_n_params());

        let body = self.parse_expression(session, reader)?;

        Self::consume_list_end(reader)?;
        self.push_expression(session, Expression::Fn { params: param_range, body }, parent_span)
    }

    fn parse_def_or_set(
        &mut self,
        session: &mut Session,
        reader: &mut Reader<'_>,
        parent_span: Span,
        is_def: bool,
    ) -> Result<ExpressionId, SParserError> {
        reader.next()?; // consume "def" / "set"

        let name = reader.next()?;
        let name = match name.value {
            Event::Symbol(name) if !ReservedSymbols::matches(&name) => session.push_symbol(&name),
            Event::Symbol(_) => return error!(UnexpectedReservedSymbol, name.span),
            _ => return error!(ExpectedSymbol, name.span),
        };

        let value = self.parse_expression(session, reader)?;

        Self::consume_list_end(reader)?;

        if is_def {
            self.push_expression(session, Expression::Def { name, value }, parent_span)
        } else {
            self.push_expression(session, Expression::Set { name, value }, parent_span)
        }
    }

    fn parse_quote(
        &mut self,
        session: &mut Session,
        reader: &mut Reader<'_>,
        _parent_span: Span,
    ) -> Result<ExpressionId, SParserError> {
        reader.next()?; // consume "quote"
        let value = self.parse_expression_quoted(session, reader)?;
        Self::consume_list_end(reader)?;
        Ok(value)
    }

    fn consume_list_end(reader: &mut Reader<'_>) -> Result<(), SParserError> {
        let event = reader.next()?;
        if event.value != Event::ListEnd {
            return error!(ExpectedListEnd, event.span);
        }
        Ok(())
    }

    fn push_expression(
        &mut self,
        session: &mut Session,
        expression: Expression,
        span: Span,
    ) -> Result<ExpressionId, SParserError> {
        Ok(session.push_expression(expression.scopy(span)))
    }

    fn push_scratch(&mut self, session: &mut Session, start: usize) -> ExpressionRange {
        let range = session.push_expressions(&self.scratch[start..]);
        self.scratch.truncate(start);
        range
    }
}

struct ReservedSymbols {}

impl ReservedSymbols {
    const DO: &'static str = "do";
    const IF: &'static str = "if";
    const FN: &'static str = "fn";
    const DEF: &'static str = "def";
    const SET: &'static str = "set";
    const QUOTE: &'static str = "quote";
    const NULL: &'static str = "null";
    const TRUE: &'static str = "true";
    const FALSE: &'static str = "false";

    pub fn matches<T: AsRef<str>>(name: &T) -> bool {
        matches!(
            name.as_ref(),
            ReservedSymbols::DO
                | ReservedSymbols::IF
                | ReservedSymbols::FN
                | ReservedSymbols::DEF
                | ReservedSymbols::SET
                | ReservedSymbols::QUOTE
                | ReservedSymbols::NULL
                | ReservedSymbols::TRUE
                | ReservedSymbols::FALSE
        )
    }
}

pub type SParserError = Spanned<ParserError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParserError {
    ReaderError(ReaderError),
    ExpectedListEnd,
    ExpectedSymbol,
    ExpectedFunctionParams,
    UnexpectedListEnd,
    UnexpectedSourceEnd,
    UnexpectedReservedSymbol,
}

impl From<Spanned<ReaderError>> for Spanned<ParserError> {
    fn from(error: Spanned<ReaderError>) -> Self {
        ParserError::ReaderError(error.value).scopy(error.span)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::expression::OwnedExpression};

    macro_rules! expressions {
        ($($kind:ident $( $args:tt )?),* $(,)?) => {
            Ok::<Vec<OwnedExpression>, SParserError>(
                vec![$(expressions!(@expr $kind $( $args )?)),*]
            )
        };

        (@expr Null) => { OwnedExpression::Null };

        (@expr Quote($kind:ident $( $args:tt )?)) => {
            OwnedExpression::Quote(Box::new(expressions!(@expr $kind $( $args )?)))
        };

        (@expr List($($kind:ident $( $args:tt )?),* $(,)?)) => {
            OwnedExpression::List(vec![$(expressions!(@expr $kind $( $args )?)),*])
        };

        (@expr Do($($kind:ident $( $args:tt )?),* $(,)?)) => {
            OwnedExpression::Do(vec![$(expressions!(@expr $kind $( $args )?)),*])
        };

        (@expr Call($call:ident $( $call_args:tt )? $(, $kind:ident $( $args:tt )?)* $(,)?)) => {
            OwnedExpression::Call {
                call: Box::new(expressions!(@expr $call $( $call_args )?)),
                args: vec![$(expressions!(@expr $kind $( $args )?)),*],
            }
        };

        (@expr If(
            $cond:ident $( $cond_args:tt )?,
            $t:ident $( $t_args:tt )?,
            $f:ident $( $f_args:tt )? $(,)?
        )) => {
            OwnedExpression::If {
                cond: Box::new(expressions!(@expr $cond $( $cond_args )?)),
                t_branch: Box::new(expressions!(@expr $t $( $t_args )?)),
                f_branch: Box::new(expressions!(@expr $f $( $f_args )?)),
            }
        };

        (@expr Function(($($param:expr),* $(,)?), $body:ident $( $body_args:tt )? $(,)?)) => {
            OwnedExpression::Fn {
                args: vec![$($param.into()),*],
                body: Box::new(expressions!(@expr $body $( $body_args )?)),
            }
        };

        (@expr Define($name:expr, $value:ident $( $value_args:tt )? $(,)?)) => {
            OwnedExpression::Def {
                name: $name.into(),
                value: Box::new(expressions!(@expr $value $( $value_args )?)),
            }
        };

        (@expr $kind:ident($value:expr)) => {OwnedExpression::$kind($value.into())};
    }

    fn parse_str_to_expressions(source: &str) -> Result<Vec<OwnedExpression>, SParserError> {
        let mut session = Session::new();
        let roots = Parser::new().parse(&mut session, &mut Reader::new(source))?;
        Ok(session.get_expressions(roots).iter().map(|id| (*id).to_owned(&session)).collect())
    }

    #[test]
    fn test_literal_null() {
        assert_eq!(parse_str_to_expressions("null"), expressions!(Null));
    }

    #[test]
    fn test_literal_integer() {
        assert_eq!(parse_str_to_expressions("42"), expressions!(Integer(42)));
    }

    #[test]
    fn test_variable() {
        assert_eq!(parse_str_to_expressions("x"), expressions!(Symbol("x")));
    }

    #[test]
    fn test_do() {
        assert_eq!(
            parse_str_to_expressions("(do (+ 1 2) (+ 3 4))"),
            expressions!(Do(
                Call(Symbol("+"), Integer(1), Integer(2)),
                Call(Symbol("+"), Integer(3), Integer(4))
            ))
        );
    }

    #[test]
    fn test_call() {
        assert_eq!(
            parse_str_to_expressions("(+ 1 2)"),
            expressions!(Call(Symbol("+"), Integer(1), Integer(2)))
        );
    }

    #[test]
    fn test_if() {
        assert_eq!(
            parse_str_to_expressions("(if true 4 2)"),
            expressions!(If(Boolean(true), Integer(4), Integer(2)))
        );
    }

    #[test]
    fn test_function() {
        assert_eq!(
            parse_str_to_expressions("(fn (a b) (+ a b))"),
            expressions!(Function(("a", "b"), Call(Symbol("+"), Symbol("a"), Symbol("b"))))
        );
    }

    #[test]
    fn test_define() {
        assert_eq!(parse_str_to_expressions("(def x 42)"), expressions!(Define("x", Integer(42))))
    }

    #[test]
    fn test_quote_a() {
        assert_eq!(
            parse_str_to_expressions("'(1 2 3)"),
            expressions!(List(Integer(1), Integer(2), Integer(3)))
        )
    }

    #[test]
    fn test_quote_b() {
        assert_eq!(
            parse_str_to_expressions("(quote (1 2 3))"),
            expressions!(List(Integer(1), Integer(2), Integer(3)))
        )
    }

    #[test]
    fn test_reader_error() {
        assert_eq!(
            parse_str_to_expressions("\""),
            Err(ReaderError::UnterminatedString.sbetween(0, 1).into())
        );
    }

    #[test]
    fn test_expected_list_end() {
        assert_eq!(
            parse_str_to_expressions("(def x 1 2)"),
            error!(ExpectedListEnd, Span::new(9, 10))
        );
    }

    #[test]
    fn test_expected_symbol() {
        assert_eq!(parse_str_to_expressions("(def 1 1)"), error!(ExpectedSymbol, Span::new(5, 6)));
    }

    #[test]
    fn test_expected_function_params() {
        assert_eq!(
            parse_str_to_expressions("(fn 1 (+ 1 2))"),
            error!(ExpectedFunctionParams, Span::new(4, 5))
        );
    }

    #[test]
    fn test_unexpected_list_end() {
        assert_eq!(parse_str_to_expressions("(def x)"), error!(UnexpectedListEnd, Span::new(6, 7)));
    }

    #[test]
    fn test_unexpected_source_end() {
        assert_eq!(
            parse_str_to_expressions("(def x"),
            error!(UnexpectedSourceEnd, Span::new(6, 7))
        );
    }

    #[test]
    fn test_unexpected_reserved_symbol_a() {
        assert_eq!(
            parse_str_to_expressions("(def if 1)"),
            error!(UnexpectedReservedSymbol, Span::new(5, 7))
        );
    }

    #[test]
    fn test_unexpected_reserved_symbol_b() {
        assert_eq!(
            parse_str_to_expressions("(fn (if) (+ if 1))"),
            error!(UnexpectedReservedSymbol, Span::new(5, 7))
        );
    }
}
