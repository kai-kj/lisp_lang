use {
    crate::{
        backend::evaluator::Evaluator,
        check,
        expression::{BindKind, Expression, ExpressionId, ExpressionRange, SExpression, SymbolId},
        frontend::{
            event::{Event, ReadError, Reader, SReadError},
            value_reader::ValueReader,
        },
        runtime::{
            builtins::make_builtins,
            error::{RuntimeError, SRuntimeError},
            session::{ParamRange, Session},
            value::UserFn,
        },
        span::{Span, Spanned, SpannedExt},
        util::Environment,
    },
    std::{
        cmp::PartialEq,
        collections::{HashMap, HashSet},
        rc::Rc,
    },
};

pub struct Parser {
    evaluator: Evaluator,
    macro_env: Rc<Environment<SymbolId, UserFn>>,
    scratch: Vec<ExpressionId>,
    fresh_names: HashMap<String, SymbolId>,
}

macro_rules! error {
    ($kind:ident, $span:expr) => {
        Err(ParseError::$kind.scopy($span))
    };
}

macro_rules! push_list_to_scratch {
    ($self:expr, $session:expr, $events:expr, $method:ident) => {{
        let scratch_start = $self.scratch.len();

        while $events.peek()?.value != Event::ListEnd {
            let arg = $self.$method($session, $events, false)?;
            let arg = $self.push_expr($session, arg)?;
            $self.scratch.push(arg);
        }

        $self.push_scratch($session, scratch_start)
    }};
}

impl Parser {
    pub fn new(
        sesh: &mut Session,
        parent_macro_env: Option<&Rc<Environment<SymbolId, UserFn>>>,
    ) -> Self {
        let macro_env = match parent_macro_env {
            Some(env) => env.child(),
            None => Environment::new(),
        };
        Self {
            evaluator: Evaluator::new(sesh, make_builtins()),
            macro_env,
            scratch: Vec::new(),
            fresh_names: HashMap::new(),
        }
    }

    pub fn parse(
        mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
    ) -> Result<ExpressionRange, SParseError> {
        while events.peek()?.value != Event::SourceEnd {
            let root = self.parse_expr(sesh, events, true)?;
            let root = self.push_expr(sesh, root)?;
            self.scratch.push(root);
        }
        Ok(self.push_scratch(sesh, 0))
    }

    fn parse_expr(
        &mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
        is_top: bool,
    ) -> Result<SExpression, SParseError> {
        let event = events.next()?;

        match event.value {
            Event::ListStart => self.parse_list(sesh, events, event.span, is_top),
            Event::ListEnd => error!(UnexpectedListEnd, event.span),
            Event::SourceEnd => error!(UnexpectedSourceEnd, event.span),
            Event::Quote => self.parse_expr_quoted(sesh, events, is_top),
            Event::Symbol(name) if name == ReservedSymbols::NULL => {
                Ok(Expression::Null.scopy(event.span))
            }
            Event::Symbol(name) if name == ReservedSymbols::TRUE => {
                Ok(Expression::Boolean(true).scopy(event.span))
            }
            Event::Symbol(name) if name == ReservedSymbols::FALSE => {
                Ok(Expression::Boolean(false).scopy(event.span))
            }
            Event::Symbol(name) if ReservedSymbols::matches(&name) => {
                error!(UnexpectedReservedSymbol, event.span)
            }
            Event::Symbol(name) if SymbolMarkers::matches_rest(&name) => {
                error!(UnexpectedRestMarker, event.span)
            }
            Event::Symbol(name) => {
                Ok(Expression::Symbol(self.make_symbol(sesh, &name)).scopy(event.span))
            }
            Event::Integer(value) => Ok(Expression::Integer(value).scopy(event.span)),
            Event::Float(value) => Ok(Expression::Float(value).scopy(event.span)),
            Event::String(value) => {
                Ok(Expression::String(sesh.push_string(&value)).scopy(event.span))
            }
        }
    }

    fn parse_expr_quoted(
        &mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
        is_top: bool,
    ) -> Result<SExpression, SParseError> {
        let event = events.next()?;

        match event.value {
            Event::ListStart => self.parse_list_quoted(sesh, events, event.span, is_top),
            Event::ListEnd => error!(UnexpectedListEnd, event.span),
            Event::Quote => {
                let quote = Expression::SymbolQ(self.make_symbol(sesh, "quote")).scopy(event.span);
                let quote = self.push_expr(sesh, quote)?;

                let value = self.parse_expr_quoted(sesh, events, is_top)?;
                let value = self.push_expr(sesh, value)?;

                let start = self.scratch.len();
                self.scratch.push(quote);
                self.scratch.push(value);
                Ok(Expression::List(self.push_scratch(sesh, start)).scopy(event.span))
            }
            Event::Symbol(v) => {
                Ok(Expression::SymbolQ(self.make_symbol(sesh, &v)).scopy(event.span))
            }
            Event::Integer(v) => Ok(Expression::Integer(v).scopy(event.span)),
            Event::Float(v) => Ok(Expression::Float(v).scopy(event.span)),
            Event::String(v) => Ok(Expression::String(sesh.push_string(&v)).scopy(event.span)),
            Event::SourceEnd => error!(UnexpectedSourceEnd, event.span),
        }
    }

    fn parse_list(
        &mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
        p_span: Span,
        is_top: bool,
    ) -> Result<SExpression, SParseError> {
        match &events.peek()?.value {
            Event::Symbol(v) if v == ReservedSymbols::QUOTE => {
                self.parse_quote(sesh, events, p_span, is_top)
            }
            Event::Symbol(v) if v == ReservedSymbols::DO => self.parse_do(sesh, events, p_span),
            Event::Symbol(v) if v == ReservedSymbols::IF => self.parse_if(sesh, events, p_span),
            Event::Symbol(v) if v == ReservedSymbols::FN => {
                let (_, req_params, rest_param, body) = self.parse_callable(sesh, events, false)?;
                Ok(Expression::Fn { req_params, rest_param, body }.scopy(p_span))
            }
            Event::Symbol(v) if v == ReservedSymbols::DEF => {
                self.parse_bind(sesh, events, p_span, BindKind::Def)
            }
            Event::Symbol(v) if v == ReservedSymbols::SET => {
                self.parse_bind(sesh, events, p_span, BindKind::Set)
            }
            Event::Symbol(v) if v == ReservedSymbols::DEF_MACRO => {
                check!(is_top, Err(ParseError::MacroDefinitionNotInTopLevel.scopy(p_span)));
                let (name, req_params, rest_param, body) =
                    self.parse_callable(sesh, events, true)?;
                let env = self.evaluator.get_root_env().clone();
                self.macro_env
                    .def(name.unwrap(), UserFn { req_params, rest_param, body, env })
                    .map_err(|_| ParseError::MacroAlreadyDefined.scopy(p_span))?;
                Ok(Expression::Null.scopy(p_span))
            }
            Event::ListEnd => {
                events.next()?; // consume list end
                Ok(Expression::Null.scopy(p_span))
            }
            _ => {
                let call = self.parse_expr(sesh, events, false)?;
                if let Expression::Symbol(call) = call.value {
                    if let Ok(call) = self.macro_env.get(call).map(|v| v.to_owned()) {
                        return self.parse_call_macro(sesh, events, &call, p_span);
                    }
                }
                self.parse_call_fn(sesh, events, call, p_span)
            }
        }
    }

    fn parse_list_quoted(
        &mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
        p_span: Span,
        is_top: bool,
    ) -> Result<SExpression, SParseError> {
        let scratch_start = self.scratch.len();

        while events.peek()?.value != Event::ListEnd {
            let child = self.parse_expr_quoted(sesh, events, is_top)?;
            let child = self.push_expr(sesh, child)?;
            self.scratch.push(child);
        }

        let body = self.push_scratch(sesh, scratch_start);

        Self::consume_list_end(events)?;
        Ok(Expression::List(body).scopy(p_span))
    }

    fn parse_quote(
        &mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
        _p_span: Span,
        is_top: bool,
    ) -> Result<SExpression, SParseError> {
        events.next()?; // consume "quote"
        let value = self.parse_expr_quoted(sesh, events, is_top)?;
        Self::consume_list_end(events)?;
        Ok(value)
    }

    fn parse_do(
        &mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
        p_span: Span,
    ) -> Result<SExpression, SParseError> {
        events.next()?; // consume "do"
        let body = push_list_to_scratch!(self, sesh, events, parse_expr);
        Self::consume_list_end(events)?;
        Ok(Expression::Do(body).scopy(p_span))
    }

    fn parse_call_fn(
        &mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
        call: SExpression,
        p_span: Span,
    ) -> Result<SExpression, SParseError> {
        let call = self.push_expr(sesh, call)?;
        let args = push_list_to_scratch!(self, sesh, events, parse_expr);
        Self::consume_list_end(events)?;
        Ok(Expression::Call { call, args }.scopy(p_span))
    }

    fn parse_call_macro(
        &mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
        call: &UserFn,
        p_span: Span,
    ) -> Result<SExpression, SParseError> {
        let args = push_list_to_scratch!(self, sesh, events, parse_expr_quoted);
        let args = sesh.get_expressions(args).to_owned();

        Self::consume_list_end(events)?;

        let expanded = self.evaluator.evaluate_macro(sesh, call, &args, p_span)?;
        let mut events = ValueReader::new(sesh, expanded, p_span)?;

        let mut parser = Self::new(sesh, Some(&self.macro_env));
        let expanded = parser.parse_expr(sesh, &mut events, false)?;

        Ok(expanded)
    }

    fn parse_if(
        &mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
        p_span: Span,
    ) -> Result<SExpression, SParseError> {
        events.next()?; // consume "if"

        let cond = self.parse_expr(sesh, events, false)?;
        let cond = self.push_expr(sesh, cond)?;

        let t_branch = self.parse_expr(sesh, events, false)?;
        let t_branch = self.push_expr(sesh, t_branch)?;

        let f_branch = self.parse_expr(sesh, events, false)?;
        let f_branch = self.push_expr(sesh, f_branch)?;

        Self::consume_list_end(events)?;
        Ok(Expression::If { cond, t_branch, f_branch }.scopy(p_span))
    }

    fn parse_bind(
        &mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
        p_span: Span,
        kind: BindKind,
    ) -> Result<SExpression, SParseError> {
        events.next()?; // consume "def" / "set"

        let name = events.next()?;
        let name = match name.value {
            Event::Symbol(name) if !ReservedSymbols::matches(&name) => {
                self.make_symbol(sesh, &name)
            }
            Event::Symbol(_) => return error!(UnexpectedReservedSymbol, name.span),
            _ => return error!(ExpectedSymbol, name.span),
        };

        let value = self.parse_expr(sesh, events, false)?;
        let value = self.push_expr(sesh, value)?;

        Self::consume_list_end(events)?;
        Ok(Expression::Bind { kind, name, value }.scopy(p_span))
    }

    fn parse_callable(
        &mut self,
        sesh: &mut Session,
        events: &mut dyn Reader,
        includes_name: bool,
    ) -> Result<(Option<SymbolId>, ParamRange, Option<SymbolId>, ExpressionId), SParseError> {
        events.next()?; // consume "fn" / "def-macro"

        let name = if includes_name {
            let event = events.next()?;
            if let Event::Symbol(name) = event.value {
                Some(self.make_symbol(sesh, &name))
            } else {
                return error!(ExpectedSymbol, event.span);
            }
        } else {
            None
        };

        let list_start = events.next()?;
        if list_start.value != Event::ListStart {
            return error!(ExpectedParameterList, list_start.span);
        }

        // make sure no recursive parsing happens before ParamRange::new()
        let params_start = sesh.get_n_params();
        let mut rest_param = None;

        let mut params = HashSet::new();

        loop {
            let event = events.next()?;
            match event.value {
                Event::ListEnd => break,
                Event::Symbol(name) if SymbolMarkers::matches_rest(&name) => {
                    let name = self.make_symbol(sesh, &name);
                    if !params.insert(name) {
                        return error!(DuplicateParameter, event.span);
                    };
                    rest_param = Some(name);

                    let list_end = events.next()?;
                    check!(
                        list_end.value == Event::ListEnd,
                        error!(ExpectedListEnd, list_end.span)
                    );
                    break;
                }
                Event::Symbol(name) if ReservedSymbols::matches(&name) => {
                    return error!(UnexpectedReservedSymbol, event.span);
                }
                Event::Symbol(name) => {
                    let param = self.make_symbol(sesh, &name);
                    if !params.insert(param) {
                        return error!(DuplicateParameter, event.span);
                    };
                    sesh.push_param(param);
                }
                _ => return error!(ExpectedSymbol, event.span),
            }
        }

        let req_params = ParamRange::new(params_start, sesh.get_n_params());
        let body = self.parse_expr(sesh, events, false)?;
        let body = self.push_expr(sesh, body)?;

        Self::consume_list_end(events)?;
        Ok((name, req_params, rest_param, body))
    }

    fn consume_list_end(events: &mut dyn Reader) -> Result<(), SParseError> {
        let event = events.next()?;
        if event.value != Event::ListEnd {
            return error!(ExpectedListEnd, event.span);
        }
        Ok(())
    }

    fn push_expr(
        &mut self,
        sesh: &mut Session,
        expression: SExpression,
    ) -> Result<ExpressionId, SParseError> {
        Ok(sesh.push_expression(expression))
    }

    fn push_scratch(&mut self, sesh: &mut Session, start: usize) -> ExpressionRange {
        let range = sesh.push_expressions(&self.scratch[start..]);
        self.scratch.truncate(start);
        range
    }

    fn make_symbol(&mut self, sesh: &mut Session, name: &str) -> SymbolId {
        if name.starts_with('#') {
            if let Some(&fresh) = self.fresh_names.get(name) {
                fresh
            } else {
                let fresh = sesh.fresh_symbol(name);
                self.fresh_names.insert(name.to_owned(), fresh);
                fresh
            }
        } else {
            sesh.push_symbol(name)
        }
    }
}

struct ReservedSymbols {}

impl ReservedSymbols {
    const DO: &'static str = "do";
    const IF: &'static str = "if";
    const FN: &'static str = "fn";
    const DEF: &'static str = "def";
    const DEF_MACRO: &'static str = "def-macro";
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
                | ReservedSymbols::DEF_MACRO
                | ReservedSymbols::SET
                | ReservedSymbols::QUOTE
                | ReservedSymbols::NULL
                | ReservedSymbols::TRUE
                | ReservedSymbols::FALSE
        )
    }
}

pub struct SymbolMarkers {}

impl SymbolMarkers {
    const REST: char = '&';
    const FRESH: char = '#';

    // pub fn matches_any<T: AsRef<str>>(name: &T) -> bool {
    //     matches!(name.as_ref().chars().next().unwrap(), Self::REST | Self::FRESH)
    // }

    pub fn matches_rest<T: AsRef<str>>(name: &T) -> bool {
        matches!(name.as_ref().chars().next().unwrap(), Self::REST)
    }

    pub fn matches_fresh<T: AsRef<str>>(name: &T) -> bool {
        matches!(name.as_ref().chars().next().unwrap(), Self::FRESH)
    }
}

pub type SParseError = Spanned<ParseError>;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    Read(Box<ReadError>),
    Expansion(Box<RuntimeError>),
    ExpectedListEnd,
    ExpectedSymbol,
    ExpectedParameterList,
    UnexpectedListEnd,
    UnexpectedSourceEnd,
    UnexpectedReservedSymbol,
    MacroDefinitionNotInTopLevel,
    MacroAlreadyDefined,
    DuplicateParameter,
    UnexpectedRestMarker,
}

impl From<SReadError> for SParseError {
    fn from(error: SReadError) -> Self {
        ParseError::Read(Box::new(error.value)).scopy(error.span)
    }
}

impl From<SRuntimeError> for SParseError {
    fn from(error: SRuntimeError) -> Self {
        ParseError::Expansion(Box::new(error.value)).scopy(error.span)
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{expression::OwnedExpression, frontend::string_reader::StringReader},
    };

    macro_rules! expressions {
        ($($kind:ident $( ( $($args:tt)* ) )?),* $(,)?) => {
            Ok(vec![$(expressions!(@expr $kind $( ( $($args)* ) )?)),*])
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

        (@expr Function(($($param:expr),* $(,)?), $body:ident $($body_args:tt)? $(,)?)) => {
            OwnedExpression::Fn {
                req_params: vec![$($param.into()),*],
                rest_param: None,
                body: Box::new(expressions!(@expr $body $( $body_args )?)),
            }
        };

         (@expr Function(($($param:expr),* $(,)?), &$rest:expr, $body:ident $($body_args:tt)? $(,)?)) => {
            OwnedExpression::Fn {
                req_params: vec![$($param.into()),*],
                rest_param: Some(format!("&{}", $rest)),
                body: Box::new(expressions!(@expr $body $( $body_args )?)),
            }
        };

        (@expr Def($name:expr, $value:ident $( $value_args:tt )? $(,)?)) => {
            OwnedExpression::Bind {
                kind: BindKind::Def,
                name: $name.into(),
                value: Box::new(expressions!(@expr $value $( $value_args )?)),
            }
        };

        (@expr $kind:ident($value:expr)) => {OwnedExpression::$kind($value.into())};
    }

    fn parse_str_to_expressions(source: &str) -> Result<Vec<OwnedExpression>, SParseError> {
        let mut sesh = Session::new();
        let roots =
            Parser::new(&mut sesh, None).parse(&mut sesh, &mut StringReader::new(source))?;
        Ok(sesh.get_expressions(roots).iter().map(|id| (*id).to_owned(&sesh)).collect())
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
    fn test_function_variadic_a() {
        assert_eq!(
            parse_str_to_expressions("(fn (&rest) null)"),
            expressions!(Function((), &"rest", Null))
        );
    }

    #[test]
    fn test_function_variadic_b() {
        assert_eq!(
            parse_str_to_expressions("(fn (a b &rest) null)"),
            expressions!(Function(("a", "b"), &"rest", Null))
        );
    }

    #[test]
    fn test_define() {
        assert_eq!(parse_str_to_expressions("(def x 42)"), expressions!(Def("x", Integer(42))))
    }

    #[test]
    fn test_quote_a() {
        assert_eq!(
            parse_str_to_expressions("'(do 1 2 3)"),
            expressions!(List(SymbolQ("do"), Integer(1), Integer(2), Integer(3)))
        )
    }

    #[test]
    fn test_quote_b() {
        assert_eq!(
            parse_str_to_expressions("(quote (do 1 2 3))"),
            expressions!(List(SymbolQ("do"), Integer(1), Integer(2), Integer(3)))
        )
    }

    #[test]
    fn test_quote_double_a() {
        assert_eq!(parse_str_to_expressions("'x"), expressions!(SymbolQ("x")))
    }

    #[test]
    fn test_quote_double_b() {
        assert_eq!(
            parse_str_to_expressions("''x"),
            expressions!(List(SymbolQ("quote"), SymbolQ("x")))
        )
    }

    #[test]
    fn test_macro_a() {
        assert_eq!(
            parse_str_to_expressions("(def-macro inc (x) (do (+ x 1))) (inc 1)"),
            expressions!(Null, Integer(2))
        )
    }

    #[test]
    fn test_macro_b() {
        assert_eq!(
            parse_str_to_expressions(
                r#"
                (def-macro if-not (x t f) (list 'if x f t))
                (if-not true 4 2)
                "#
            ),
            expressions!(Null, If(Boolean(true), Integer(2), Integer(4)))
        )
    }

    #[test]
    fn test_reader_error() {
        assert_eq!(
            parse_str_to_expressions("\""),
            Err(ReadError::UnterminatedString.sbetween(0, 1).into())
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
            error!(ExpectedParameterList, Span::new(4, 5))
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
