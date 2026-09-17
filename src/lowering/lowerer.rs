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
        syntax: &SSyntax,
    ) -> Result<SExpression, SLowererError> {
        Ok(match &syntax.value {
            Syntax::Boolean(value) => Expression::Literal(Value::Boolean(*value)),
            Syntax::Integer(value) => Expression::Literal(Value::Integer(*value)),
            Syntax::Float(value) => Expression::Literal(Value::Float(*value)),
            Syntax::String(value) => Expression::Literal(Value::String(value.clone())),
            Syntax::Symbol(symbol) => match symbol_table.get_core_symbol(symbol) {
                Some(CoreSymbol::Null) => Expression::Literal(Value::Null),
                Some(CoreSymbol::True) => Expression::Literal(Value::Boolean(true)),
                Some(CoreSymbol::False) => Expression::Literal(Value::Boolean(false)),
                Some(_) => {
                    return Err(LowererError::InvalidCallName.span(syntax.span));
                }
                None => Expression::Variable(*symbol),
            },
            Syntax::List(items) => {
                return self.lower_list(symbol_table, &items, syntax.span);
            }
        }
        .span(syntax.span))
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
        let head_span = head.span;
        let Syntax::Symbol(head) = head.value else {
            return Err(LowererError::InvalidCallName.span(head_span));
        };
        match symbol_table.get_core_symbol(&head) {
            Some(CoreSymbol::If) => {
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
            Some(CoreSymbol::Lambda) => {
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
            Some(CoreSymbol::Define) => {
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
            Some(CoreSymbol::Quote) => Err(LowererError::QuoteUnsupported.span(parent_span)),
            Some(_) => {
                return Err(LowererError::InvalidCallName.span(head_span));
            }
            None => Ok(Expression::Call {
                name: head,
                args: rest
                    .iter()
                    .map(|expr| self.lower_expression(symbol_table, expr))
                    .collect::<Result<Vec<_>, _>>()?,
            }
            .span(parent_span)),
        }
    }
}

pub type SLowererError = Spanned<LowererError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LowererError {
    ParseError(ParserError),
    InvalidIf,
    InvalidLambda,
    InvalidLambdaArgs,
    InvalidLambdaArg,
    InvalidLambdaBody,
    InvalidDefine,
    InvalidDefineName,
    QuoteUnsupported,
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
}
