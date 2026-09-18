use crate::{
    lexing::{
        lexer::{Lexer, LexerError, SLexerError},
        token::Token,
    },
    parsing::syntax::{SSyntax, Syntax},
    span::{Spanned, SpannedExt},
    symbol::SymbolTable,
};

pub struct Parser<'table> {
    symbol_table: &'table mut SymbolTable,
}

impl<'table> Parser<'table> {
    pub fn new(symbol_table: &'table mut SymbolTable) -> Self {
        Self { symbol_table }
    }

    pub fn parse(&mut self, lexer: &mut Lexer) -> Result<Vec<SSyntax>, SParserError> {
        let mut expressions = Vec::new();
        loop {
            match Self::parse_expression(&mut self.symbol_table, lexer) {
                Ok(Some(expression)) => expressions.push(expression),
                Ok(None) => return Ok(expressions),
                Err(err) => return Err(err),
            }
        }
    }

    pub fn parse_expression(
        symbol_table: &mut SymbolTable,
        lexer: &mut Lexer,
    ) -> Result<Option<SSyntax>, SParserError> {
        let first_token = lexer.next()?;
        let mut last_span = first_token.span;

        let kind = match first_token.value {
            Token::End => return Ok(None),
            Token::ParenLeft => {
                let mut items = Vec::new();
                loop {
                    match lexer.peek()? {
                        next_token if next_token.value == Token::ParenRight => {
                            lexer.next()?;
                            last_span = next_token.span;
                            break;
                        }
                        next_token if next_token.value == Token::End => {
                            return Err(ParserError::MissingRightParen
                                .span_join(first_token.span, next_token.span));
                        }
                        _ => {
                            if let Some(expression) = Self::parse_expression(symbol_table, lexer)? {
                                items.push(expression);
                            }
                        }
                    }
                }
                Syntax::List(items)
            }
            Token::ParenRight => {
                return Err(ParserError::UnexpectedRightParen.span(first_token.span));
            }
            Token::Quote => Syntax::List(vec![
                Syntax::Symbol(symbol_table.add_symbol("quote")).span(first_token.span),
                Self::parse_expression(symbol_table, lexer)?
                    .ok_or(ParserError::NothingToQuote.span(first_token.span))?,
            ]),
            Token::Symbol(name) => match name {
                "if" => Syntax::If,
                "fn" => Syntax::Function,
                "define" => Syntax::Define,
                "quote" => Syntax::Quote,
                "null" => Syntax::Null,
                "true" => Syntax::Boolean(true),
                "false" => Syntax::Boolean(false),
                _ => Syntax::Symbol(symbol_table.add_symbol(name)),
            },
            Token::Integer(value) => Syntax::Integer(value),
            Token::Float(value) => Syntax::Float(value),
            Token::String(value) => {
                let mut string = String::with_capacity(value.len());
                let mut chars = value.chars();
                let mut off = 0;

                while let Some(c) = chars.next() {
                    if c != '\\' {
                        string.push(c);
                        off += c.len_utf8();
                        continue;
                    }
                    let escaped = match chars.next() {
                        Some('n') => Ok('\n'),
                        Some('r') => Ok('\r'),
                        Some('t') => Ok('\t'),
                        Some('0') => Ok('\0'),
                        Some('\\') => Ok('\\'),
                        Some('"') => Ok('"'),
                        _ => {
                            if let Some(span) = first_token.span {
                                Err(ParserError::UnexpectedEscape
                                    .span_between(span.start + off + 1, span.start + off + 2))
                            } else {
                                Err(ParserError::UnexpectedEscape.span_none())
                            }
                        }
                    }?;
                    string.push(escaped);
                }
                Syntax::String(string)
            }
        };
        Ok(Some(kind.span_join(first_token.span, last_span)))
    }
}

pub type SParserError = Spanned<ParserError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParserError {
    LexerError(LexerError),
    MissingRightParen,
    UnexpectedRightParen,
    NothingToQuote,
    UnexpectedEscape,
}

impl From<SLexerError> for SParserError {
    fn from(value: SLexerError) -> Self {
        SParserError { value: ParserError::LexerError(value.value), span: value.span }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::display::WithDisplayContextExt};

    macro_rules! syntax_list {
        ($($kind:ident ($($args:tt)*)),* $(,)?) => {{
            let mut _symbol_table = SymbolTable::new();
            Ok(syntax_list!(_symbol_table; $($kind($($args)*)),*))
        }};

        ($table:ident; $($kind:ident ($($args:tt)*)),* $(,)?) => {{
            let _symbol_table = &mut $table;
            vec![$(syntax_list!(@kind _symbol_table; $kind($($args)*))),*]
        }};

        (@kind $table:ident; Symbol($value:expr)) => {
            Syntax::Symbol($table.add_symbol($value)).span_none()
        };

        (@kind $table:ident; List($($kind:ident ($($args:tt)*)),* $(,)?)) => {
            Syntax::List(vec![$(syntax_list!(@kind $table; $kind($($args)*))),*]).span_none()
        };

        (@kind $table:ident; $kind:ident($value:expr)) => {
            Syntax::$kind($value.into()).span_none()
        };
    }

    fn parse_str(source: &str) -> Result<Vec<SSyntax>, SParserError> {
        let mut symbol_table = SymbolTable::new();
        let mut lexer = Lexer::new(source);

        let mut parser = Parser::new(&mut symbol_table);
        let syntax_list = parser.parse(&mut lexer)?;

        for syntax in &syntax_list {
            println!("{}", syntax.with_symbols(&symbol_table).set_indent(2));
        }

        Ok(syntax_list)
    }

    #[test]
    fn test_parse_list_nested() {
        assert_eq!(
            parse_str(r#"(+ 1 (* 2 3))"#),
            syntax_list!(List(Symbol("+"), Integer(1), List(Symbol("*"), Integer(2), Integer(3))))
        )
    }

    #[test]
    fn test_parse_list_quote() {
        assert_eq!(
            parse_str(r#"'(1 2)"#),
            syntax_list!(List(Symbol("quote"), List(Integer(1), Integer(2))))
        )
    }

    #[test]
    fn test_parse_symbol() {
        assert_eq!(parse_str(r#"foo"#), syntax_list!(Symbol("foo")))
    }

    #[test]
    fn test_parse_string() {
        assert_eq!(parse_str(r#""Hello, world!""#), syntax_list!(String("Hello, world!")))
    }

    #[test]
    fn test_parse_string_escaped_a() {
        assert_eq!(parse_str(r#""Hello, \"world\"!""#), syntax_list!(String(r#"Hello, "world"!"#)))
    }

    #[test]
    fn test_parse_string_escaped_b() {
        assert_eq!(parse_str(r#""Hello,\tworld!""#), syntax_list!(String(r#"Hello,	world!"#)))
    }

    #[test]
    fn test_parse_string_escaped_c() {
        assert_eq!(parse_str(r#""Hello, \\world!""#), syntax_list!(String(r#"Hello, \world!"#)))
    }

    #[test]
    fn test_list_span_a() {
        assert_eq!(parse_str(r#"(    )"#), Ok(vec![Syntax::List(vec![]).span_between(0, 6)]),)
    }

    #[test]
    fn test_list_span_b() {
        assert_eq!(parse_str(r#"(    ) "#), Ok(vec![Syntax::List(vec![]).span_between(0, 6)]),)
    }

    #[test]
    fn test_lexer_error() {
        assert_eq!(
            parse_str(r#""Hello, world!"#),
            Err(LexerError::UnterminatedString.span_between(0, 14).into())
        );
    }

    #[test]
    fn test_parse_missing_right_paren() {
        assert_eq!(parse_str(r#"()(()"#), Err(ParserError::MissingRightParen.span_between(2, 5)))
    }

    #[test]
    fn test_parse_unexpected_right_paren() {
        assert_eq!(
            parse_str(r#"()(()))"#),
            Err(ParserError::UnexpectedRightParen.span_between(6, 7))
        )
    }

    #[test]
    fn test_parse_nothing_to_quote() {
        assert_eq!(parse_str(r#"'()'"#), Err(ParserError::NothingToQuote.span_between(3, 4)))
    }

    #[test]
    fn test_parse_unexpected_escape() {
        assert_eq!(
            parse_str(r#""Hello, \x world!""#),
            Err(ParserError::UnexpectedEscape.span_between(8, 9))
        )
    }
}
