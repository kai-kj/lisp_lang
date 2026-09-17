use crate::prelude::*;

pub struct Lowerer {}

impl Lowerer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn lower(
        &self,
        symbol_table: &mut SymbolTable,
        syntax: &Vec<SSyntax>,
    ) -> Result<Vec<SExpression>, SLowererError> {
        syntax.into_iter().map(|syntax| self.lower_expression(symbol_table, &syntax)).collect()
    }

    pub fn lower_expression(
        &self,
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
            Syntax::List(v) => self.lower_list(symbol_table, &v, s.span),
            _ => Err(LowererError::UnexpectedExpression.span(s.span)),
        }
    }

    fn lower_list(
        &self,
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
                    cond: Box::new(self.lower_expression(symbol_table, condition)?),
                    t_branch: Box::new(self.lower_expression(symbol_table, then_branch)?),
                    f_branch: Box::new(self.lower_expression(symbol_table, else_branch)?),
                }
                .span(parent_span))
            }
            Syntax::Lambda => {
                let [args, body @ ..] = rest else {
                    return Err(LowererError::InvalidLambda.span(parent_span));
                };
                let Syntax::List(args) = &args.value else {
                    return Err(LowererError::InvalidLambdaArgs.span(args.span));
                };
                let args = args
                    .iter()
                    .map(|arg| match arg.value {
                        Syntax::Symbol(symbol) => Ok(symbol),
                        _ => Err(LowererError::InvalidLambdaArg.span(arg.span)),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if body.is_empty() {
                    return Err(LowererError::InvalidLambdaBody.span(parent_span));
                }
                let body = body
                    .iter()
                    .map(|expr| self.lower_expression(symbol_table, expr))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Expression::Lambda { args, body }.span(parent_span))
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
                    value: Box::new(self.lower_expression(symbol_table, value)?),
                }
                .span(parent_span))
            }
            Syntax::Quote => panic!("quote is not supported yet"),
            Syntax::Symbol(name) => Ok(Expression::Call {
                name,
                args: rest
                    .iter()
                    .map(|expr| self.lower_expression(symbol_table, expr))
                    .collect::<Result<Vec<_>, _>>()?,
            }
            .span(parent_span)),
            _ => Err(LowererError::InvalidCallName.span(head.span)),
        }
    }
}

pub type SLowererError = Spanned<LowererError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LowererError {
    ParseError(ParserError),
    UnexpectedExpression,
    InvalidIf,
    InvalidLambda,
    InvalidLambdaArgs,
    InvalidLambdaArg,
    InvalidLambdaBody,
    InvalidDefine,
    InvalidDefineName,
    InvalidCallName,
}

impl From<SParserError> for SLowererError {
    fn from(value: SParserError) -> Self {
        SLowererError { value: LowererError::ParseError(value.value), span: value.span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lower_str(source: &str) -> Result<Vec<SExpression>, SLowererError> {
        let mut symbol_table = SymbolTable::new();
        let mut parser = Parser::new(source);
        let lowerer = Lowerer::new();
        let syntax_list = parser.parse(&mut symbol_table)?;
        let expression_list = lowerer.lower(&mut symbol_table, &syntax_list)?;

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
            expression_list!(Call("+"; Literal(Integer(1)), Literal(Integer(2))))
        );
    }

    #[test]
    fn test_lower_if() {
        assert_eq!(
            lower_str("(if true 4 2)"),
            expression_list!(If(Literal(Boolean(true)); Literal(Integer(4)); Literal(Integer(2))))
        );
    }

    #[test]
    fn test_lower_lambda() {
        assert_eq!(
            lower_str("(lambda (a b) (print a) (+ a b))"),
            expression_list!(Lambda(("a", "b"); Call("print"; Variable("a")), Call("+"; Variable("a"), Variable("b"))))
        );
    }

    #[test]
    fn test_lower_define() {
        assert_eq!(lower_str("(define x 42)"), expression_list!(Define("x"; Literal(Integer(42)))))
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
    fn test_lower_invalid_lambda() {
        assert_eq!(lower_str("(lambda)"), Err(LowererError::InvalidLambda.span_between(0, 8)));
    }

    #[test]
    fn test_lower_invalid_lambda_args() {
        assert_eq!(
            lower_str("(lambda a a)"),
            Err(LowererError::InvalidLambdaArgs.span_between(8, 9))
        );
    }

    #[test]
    fn test_lower_invalid_lambda_arg() {
        assert_eq!(
            lower_str("(lambda (3) 3)"),
            Err(LowererError::InvalidLambdaArg.span_between(9, 10))
        );
    }

    #[test]
    fn test_lower_invalid_lambda_body() {
        assert_eq!(
            lower_str("(lambda (a))"),
            Err(LowererError::InvalidLambdaBody.span_between(0, 12))
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
        assert_eq!(
            lower_str("(42)"),
            Err(LowererError::InvalidCallName.span_between(1, 3))
        );
    }
}
