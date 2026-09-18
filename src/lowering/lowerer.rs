use crate::{
    lowering::expression::{Expression, SExpression},
    parsing::{
        parser::{ParserError, SParserError},
        syntax::{SSyntax, Syntax},
    },
    runtime::value::Value,
    span::{Span, Spanned, SpannedExt},
    symbol::SymbolTable,
};

pub struct Lowerer<'table> {
    symbol_table: &'table mut SymbolTable,
}

impl<'table> Lowerer<'table> {
    pub fn new(symbol_table: &'table mut SymbolTable) -> Self {
        Self { symbol_table }
    }

    pub fn lower(&mut self, syntax: &Vec<SSyntax>) -> Result<Vec<SExpression>, SLowererError> {
        syntax
            .into_iter()
            .map(|syntax| Self::lower_expression(&mut self.symbol_table, &syntax))
            .collect()
    }

    pub fn lower_expression(
        symbol_table: &mut SymbolTable,
        s: &SSyntax,
    ) -> Result<SExpression, SLowererError> {
        match &s.value {
            Syntax::Null => Ok(Expression::Literal(Value::Null).span(s.span)),
            Syntax::Symbol(v) => Ok(Expression::Variable(*v).span(s.span)),
            Syntax::Boolean(v) => Ok(Expression::Literal(Value::Boolean(*v)).span(s.span)),
            Syntax::Integer(v) => Ok(Expression::Literal(Value::Integer(*v)).span(s.span)),
            Syntax::Float(v) => Ok(Expression::Literal(Value::Float(*v)).span(s.span)),
            Syntax::String(v) => Ok(Expression::Literal(Value::String(v.clone())).span(s.span)),
            Syntax::List(v) => Self::lower_list(symbol_table, &v, s.span),
            _ => Err(LowererError::UnexpectedExpression.span(s.span)),
        }
    }

    fn lower_list(
        symbol_table: &mut SymbolTable,
        items: &[SSyntax],
        parent_span: Option<Span>,
    ) -> Result<SExpression, SLowererError> {
        let [head, rest @ ..] = items else {
            return Ok(Expression::Literal(Value::Null).span(parent_span));
        };

        match head.value {
            Syntax::If => {
                let [condition, then_branch, else_branch] = rest else {
                    return Err(LowererError::InvalidIf.span(parent_span));
                };
                Ok(Expression::If {
                    cond: Box::new(Self::lower_expression(symbol_table, condition)?),
                    t_branch: Box::new(Self::lower_expression(symbol_table, then_branch)?),
                    f_branch: Box::new(Self::lower_expression(symbol_table, else_branch)?),
                }
                    .span(parent_span))
            }
            Syntax::Function => {
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
                let body = body
                    .iter()
                    .map(|expr| Self::lower_expression(symbol_table, expr))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Expression::Function { args, body }.span(parent_span))
            }
            Syntax::Define => {
                let [name, value] = rest else {
                    return Err(LowererError::InvalidDefine.span(parent_span));
                };
                let Syntax::Symbol(name) = name.value else {
                    return Err(LowererError::InvalidDefineName.span(name.span));
                };
                Ok(Expression::Define {
                    name,
                    value: Box::new(Self::lower_expression(symbol_table, value)?),
                }.span(parent_span))
            }
            Syntax::Quote => panic!("quote is not supported yet"),
            Syntax::Symbol(_) | Syntax::List(_) => {
                Ok(Expression::Call {
                    call: Box::new(Self::lower_expression(symbol_table, head)?),
                    args: rest
                        .iter()
                        .map(|expr| Self::lower_expression(symbol_table, expr))
                        .collect::<Result<Vec<_>, _>>()?,
                }.span(parent_span))
            }
            _ => Err(LowererError::InvalidCall.span(head.span)),
        }
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
            parsing::parser::{Parser, ParserError},
            runtime::value::Value,
            span::SpannedExt,
            symbol::SymbolTable,
        },
    };

    macro_rules! expression_list {
        ($($kind:ident $args:tt),* $(,)?) => {{
            let mut _symbol_table = SymbolTable::new();
            Ok(expression_list!(_symbol_table; $($kind $args),*))
        }};

        ($table:ident; $($kind:ident $args:tt),* $(,)?) => {{
            let _symbol_table = &mut $table;
            vec![$(expression_list!(@expr _symbol_table; $kind $args)),*]
        }};

        (@expr $table:ident; Literal(Null)) => { Expression::Literal(Value::Null).span_none() };

        (@expr $table:ident; Literal(Symbol($value:expr))) => {
            Expression::Literal(Value::Symbol($table.add_symbol($value))).span_none()
        };

        (@expr $table:ident; Literal($kind:ident($value:expr))) => {
            Expression::Literal(Value::$kind($value).into()).span_none()
        };

        (@expr $table:ident; Variable($name:expr)) => {
            Expression::Variable($table.add_symbol($name)).span_none()
        };

        (@expr $table:ident; Call($call_kind:ident $call_args:tt $(, $kind:ident $args:tt)* $(,)?)) => {
            Expression::Call {
                call: Box::new(expression_list!(@expr $table; $call_kind $call_args)),
                args: vec![$(expression_list!(@expr $table; $kind $args)),*],
            }.span_none()
        };

        (@expr $table:ident; If(
            $cond_kind:ident $cond_args:tt,
            $t_kind:ident $t_args:tt,
            $f_kind:ident $f_args:tt
        )) => {
            Expression::If {
                cond: Box::new(expression_list!(@expr $table; $cond_kind $cond_args)),
                t_branch: Box::new(expression_list!(@expr $table; $t_kind $t_args)),
                f_branch: Box::new(expression_list!(@expr $table; $f_kind $f_args)),
            }.span_none()
        };

        (@expr $table:ident; Function(
            ($($arg:expr),* $(,)?),
            $($kind:ident $args:tt),* $(,)?
        )) => {
            Expression::Function {
                args: vec![
                    $($table.add_symbol($arg)),*
                ],
                body: vec![
                    $(expression_list!(@expr $table; $kind $args)),*
                ],
            }.span_none()
        };

        (@expr $table:ident; Define(
            $name:expr,
            $kind:ident $args:tt
        )) => {
            Expression::Define {
                name: $table.add_symbol($name),
                value: Box::new(
                    expression_list!(@expr $table; $kind $args)
                ),
            }.span_none()
        };
    }

    fn lower_str(source: &str) -> Result<Vec<SExpression>, SLowererError> {
        let mut symbol_table = SymbolTable::new();
        let mut lexer = Lexer::new(source);

        let mut parser = Parser::new(&mut symbol_table);
        let syntax_list = parser.parse(&mut lexer)?;

        let mut lowerer = Lowerer::new(&mut symbol_table);
        let expression_list = lowerer.lower(&syntax_list)?;

        for expression in &expression_list {
            println!("{}", expression.with_symbols(&symbol_table).set_indent(2));
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
        assert_eq!(lower_str("(define x 42)"), expression_list!(Define("x", Literal(Integer(42)))))
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
        assert_eq!(lower_str("(define a)"), Err(LowererError::InvalidDefine.span_between(0, 10)));
    }

    #[test]
    fn test_lower_invalid_define_name() {
        assert_eq!(
            lower_str(r#"(define "a" 42)"#),
            Err(LowererError::InvalidDefineName.span_between(8, 11))
        );
    }

    #[test]
    fn test_lower_invalid_call_name() {
        assert_eq!(lower_str("(42)"), Err(LowererError::InvalidCall.span_between(1, 3)));
    }
}
