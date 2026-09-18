use crate::{
    lowering::expression::{Expression, SExpression},
    parsing::{
        parser::{ParserError, SParserError},
        syntax::{SSyntax, Syntax},
    },
    runtime::value::Value,
    span::{Span, Spanned, SpannedExt},
};

pub fn lower<'s>(syntax: &Vec<SSyntax<'s>>) -> Result<Vec<SExpression<'s>>, SLowererError> {
    syntax.into_iter().map(|syntax| lower_expression(&syntax)).collect()
}

fn lower_expression<'s>(s: &SSyntax<'s>) -> Result<SExpression<'s>, SLowererError> {
    match &s.value {
        Syntax::Symbol(v) => match *v {
            "null" => Ok(Expression::Literal(Value::Null).span(s.span)),
            "true" => Ok(Expression::Literal(Value::Boolean(true)).span(s.span)),
            "false" => Ok(Expression::Literal(Value::Boolean(false)).span(s.span)),
            "if" | "fn" | "def" | "quote" => Err(LowererError::UnexpectedExpression.span(s.span)),
            _ => Ok(Expression::Variable(*v).span(s.span)),
        },
        Syntax::Integer(v) => Ok(Expression::Literal(Value::Integer(*v)).span(s.span)),
        Syntax::Float(v) => Ok(Expression::Literal(Value::Float(*v)).span(s.span)),
        Syntax::String(v) => Ok(Expression::Literal(Value::String(v.clone())).span(s.span)),
        Syntax::List(v) => lower_list(&v, s.span),
        _ => Err(LowererError::UnexpectedExpression.span(s.span)),
    }
}

fn lower_list<'s>(
    items: &[SSyntax<'s>],
    parent_span: Option<Span>,
) -> Result<SExpression<'s>, SLowererError> {
    let [head, rest @ ..] = items else {
        return Ok(Expression::Literal(Value::Null).span(parent_span));
    };

    match head.value {
        Syntax::Symbol("if") => {
            let [condition, then_branch, else_branch] = rest else {
                return Err(LowererError::InvalidIf.span(parent_span));
            };
            Ok(Expression::If {
                cond: Box::new(lower_expression(condition)?),
                t_branch: Box::new(lower_expression(then_branch)?),
                f_branch: Box::new(lower_expression(else_branch)?),
            }
            .span(parent_span))
        }
        Syntax::Symbol("fn") => {
            let [args, body @ ..] = rest else {
                return Err(LowererError::InvalidFunction.span(parent_span));
            };
            let Syntax::List(args) = &args.value else {
                return Err(LowererError::InvalidFunctionArgs.span(args.span));
            };
            let args = args
                .iter()
                .map(|arg| match arg.value {
                    Syntax::Symbol(symbol) => Ok(symbol),
                    _ => Err(LowererError::InvalidFunctionArg.span(arg.span)),
                })
                .collect::<Result<Vec<_>, _>>()?;
            if body.is_empty() {
                return Err(LowererError::InvalidFunctionBody.span(parent_span));
            }
            let body =
                body.iter().map(|expr| lower_expression(expr)).collect::<Result<Vec<_>, _>>()?;
            Ok(Expression::Function { args, body }.span(parent_span))
        }
        Syntax::Symbol("def") => {
            let [name, value] = rest else {
                return Err(LowererError::InvalidDefine.span(parent_span));
            };
            let Syntax::Symbol(name) = name.value else {
                return Err(LowererError::InvalidDefineName.span(name.span));
            };
            Ok(Expression::Define { name, value: Box::new(lower_expression(value)?) }
                .span(parent_span))
        }
        Syntax::Symbol("quote") => panic!("quote is not supported yet"),
        Syntax::Symbol(_) | Syntax::List(_) => Ok(Expression::Call {
            call: Box::new(lower_expression(head)?),
            args: rest.iter().map(|expr| lower_expression(expr)).collect::<Result<Vec<_>, _>>()?,
        }
        .span(parent_span)),
        _ => Err(LowererError::InvalidCall.span(head.span)),
    }
}

pub type SLowererError = Spanned<LowererError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LowererError {
    ParseError(ParserError),
    UnexpectedExpression,
    InvalidIf,
    InvalidFunction,
    InvalidFunctionArgs,
    InvalidFunctionArg,
    InvalidFunctionBody,
    InvalidDefine,
    InvalidDefineName,
    InvalidCall,
}

impl From<SParserError> for SLowererError {
    fn from(value: SParserError) -> Self {
        SLowererError { value: LowererError::ParseError(value.value), span: value.span }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            display::WithDisplayContextExt,
            lexing::lexer::Lexer,
            parsing::parser::{ParserError, parse},
            runtime::value::Value,
            span::SpannedExt,
        },
    };

    macro_rules! expression_list {
        ($($kind:ident $args:tt),* $(,)?) => {{
            Ok(vec![$(expression_list!(@expr $kind $args)),*])
        }};

        (@expr Literal(Null)) => { Expression::Literal(Value::Null).span_none() };

        (@expr Literal($kind:ident($value:expr))) => {
            Expression::Literal(Value::$kind($value).into()).span_none()
        };

        (@expr Variable($name:expr)) => { Expression::Variable($name).span_none() };

        (@expr Call($call_kind:ident $call_args:tt $(, $kind:ident $args:tt)* $(,)?)) => {
            Expression::Call {
                call: Box::new(expression_list!(@expr $call_kind $call_args)),
                args: vec![$(expression_list!(@expr $kind $args)),*],
            }.span_none()
        };

        (@expr If(
            $cond_kind:ident $cond_args:tt,
            $t_kind:ident $t_args:tt,
            $f_kind:ident $f_args:tt
        )) => {
            Expression::If {
                cond: Box::new(expression_list!(@expr $cond_kind $cond_args)),
                t_branch: Box::new(expression_list!(@expr $t_kind $t_args)),
                f_branch: Box::new(expression_list!(@expr $f_kind $f_args)),
            }.span_none()
        };

        (@expr Function(
            ($($arg:expr),* $(,)?),
            $($kind:ident $args:tt),* $(,)?
        )) => {
            Expression::Function {
                args: vec![ $($arg),* ],
                body: vec![ $(expression_list!(@expr $kind $args)),* ],
            }.span_none()
        };

        (@expr Define(
            $name:expr,
            $kind:ident $args:tt
        )) => {
            Expression::Define {
                name: $name,
                value: Box::new(expression_list!(@expr $kind $args)),
            }.span_none()
        };
    }

    fn lower_str(source: &str) -> Result<Vec<SExpression>, SLowererError> {
        let mut lexer = Lexer::new(source);
        let syntax_list = parse(&mut lexer)?;
        let expression_list = lower(&syntax_list)?;

        for expression in &expression_list {
            println!("{}", expression.disp().set_indent(2));
        }

        Ok(expression_list)
    }

    #[test]
    fn test_lower_literal_null() {
        assert_eq!(lower_str("null"), expression_list!(Literal(Null)));
    }

    #[test]
    fn test_lower_literal_integer() {
        assert_eq!(lower_str("42"), expression_list!(Literal(Integer(42))));
    }

    #[test]
    fn test_lower_variable() {
        assert_eq!(lower_str("x"), expression_list!(Variable("x")));
    }

    #[test]
    fn test_lower_call() {
        assert_eq!(
            lower_str("(+ 1 2)"),
            expression_list!(Call(Variable("+"), Literal(Integer(1)), Literal(Integer(2))))
        );
    }

    #[test]
    fn test_lower_if() {
        assert_eq!(
            lower_str("(if true 4 2)"),
            expression_list!(If(Literal(Boolean(true)), Literal(Integer(4)), Literal(Integer(2))))
        );
    }

    #[test]
    fn test_lower_function() {
        assert_eq!(
            lower_str("(fn (a b) (print a) (+ a b))"),
            expression_list!(Function(
                ("a", "b"),
                Call(Variable("print"), Variable("a")),
                Call(Variable("+"), Variable("a"), Variable("b"))
            ))
        );
    }

    #[test]
    fn test_lower_define() {
        assert_eq!(lower_str("(def x 42)"), expression_list!(Define("x", Literal(Integer(42)))))
    }

    #[test]
    fn test_lower_parser_error() {
        assert_eq!(
            lower_str("("),
            Err(LowererError::ParseError(ParserError::MissingRightParen).span_between(0, 0))
        );
    }

    #[test]
    fn test_lower_unexpected_expression() {
        assert_eq!(lower_str("if"), Err(LowererError::UnexpectedExpression.span_between(0, 2)));
    }

    #[test]
    fn test_lower_invalid_if_a() {
        assert_eq!(lower_str("(if true)"), Err(LowererError::InvalidIf.span_between(0, 9)));
    }

    #[test]
    fn test_lower_invalid_if_b() {
        assert_eq!(lower_str("(if true 4)"), Err(LowererError::InvalidIf.span_between(0, 11)));
    }

    #[test]
    fn test_lower_invalid_function() {
        assert_eq!(lower_str("(fn)"), Err(LowererError::InvalidFunction.span_between(0, 4)));
    }

    #[test]
    fn test_lower_invalid_function_args() {
        assert_eq!(
            lower_str("(fn a a)"),
            Err(LowererError::InvalidFunctionArgs.span_between(4, 5))
        );
    }

    #[test]
    fn test_lower_invalid_function_arg() {
        assert_eq!(
            lower_str("(fn (3) 3)"),
            Err(LowererError::InvalidFunctionArg.span_between(5, 6))
        );
    }

    #[test]
    fn test_lower_invalid_function_body() {
        assert_eq!(
            lower_str("(fn (a))"),
            Err(LowererError::InvalidFunctionBody.span_between(0, 8))
        );
    }

    #[test]
    fn test_lower_invalid_define() {
        assert_eq!(lower_str("(def a)"), Err(LowererError::InvalidDefine.span_between(0, 7)));
    }

    #[test]
    fn test_lower_invalid_define_name() {
        assert_eq!(
            lower_str(r#"(def "a" 42)"#),
            Err(LowererError::InvalidDefineName.span_between(5, 8))
        );
    }

    #[test]
    fn test_lower_invalid_call_name() {
        assert_eq!(lower_str("(42)"), Err(LowererError::InvalidCall.span_between(1, 3)));
    }
}
