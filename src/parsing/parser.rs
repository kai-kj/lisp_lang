use crate::{
    lexing::{
        lexer::{Lexer, LexerError, SLexerError},
        token::Token,
    },
    parsing::syntax::{
        StringStore, SymbolId, SymbolStore, Syntax, SyntaxId, SyntaxRange, SyntaxStore, SyntaxTree,
    },
    span::{Spanned, SpannedExt},
};

pub struct Parser {
    syntaxes: SyntaxStore,
    symbols: SymbolStore,
    strings: StringStore,
    scratch: Vec<SyntaxId>,
    quote: SymbolId,
}

impl Parser {
    pub fn new() -> Self {
        let mut symbols = SymbolStore::new();
        let quote = symbols.push("quote");

        Self {
            syntaxes: SyntaxStore::new(),
            symbols,
            strings: StringStore::new(),
            scratch: Vec::new(),
            quote,
        }
    }

    pub fn parse(mut self, lexer: &mut Lexer<'_>) -> Result<SyntaxTree, SParserError> {
        while lexer.peek()?.value != Token::End {
            let root = self.parse_expression(lexer)?;
            self.scratch.push(root);
        }

        let roots = self.scratch_push(0);

        Ok(SyntaxTree {
            roots,
            syntaxes: self.syntaxes,
            symbols: self.symbols,
            strings: self.strings,
        })
    }

    fn parse_expression(&mut self, lexer: &mut Lexer<'_>) -> Result<SyntaxId, SParserError> {
        let t_start = lexer.next()?;

        let value = match t_start.value {
            Token::End => {
                return Err(ParserError::NothingToQuote.sinherit(&t_start));
            }
            Token::ParenRight => {
                return Err(ParserError::UnexpectedRightParen.sinherit(&t_start));
            }
            Token::ParenLeft => {
                let scratch_start = self.scratch.len();

                loop {
                    let t_next = lexer.peek()?;
                    match t_next.value {
                        Token::ParenRight => {
                            lexer.next()?;
                            let children = self.scratch_push(scratch_start);
                            return Ok(self
                                .syntaxes
                                .push(Syntax::List(children).sjoin(&t_start, &t_next)));
                        }
                        Token::End => {
                            return Err(ParserError::MissingRightParen.sjoin(&t_start, &t_next));
                        }
                        _ => {
                            let child = self.parse_expression(lexer)?;
                            self.scratch.push(child);
                        }
                    }
                }
            }
            Token::Quote => {
                if lexer.peek()?.value == Token::End {
                    return Err(ParserError::NothingToQuote.sinherit(&t_start));
                }
                let quoted = self.parse_expression(lexer)?;
                let quotee = self.syntaxes.push(Syntax::Symbol(self.quote).sinherit(&t_start));
                Syntax::List(self.syntaxes.push_children(&[quotee, quoted]))
            }
            Token::Symbol(v) => Syntax::Symbol(self.symbols.push(v)),
            Token::Integer(v) => Syntax::Integer(v),
            Token::Float(v) => Syntax::Float(v),
            Token::String(raw) => {
                let mut res = String::with_capacity(raw.len());
                let mut chars = raw.chars();

                while let Some(c) = chars.next() {
                    if c != '\\' {
                        res.push(c);
                        continue;
                    }
                    res.push(match chars.next() {
                        Some('n') => '\n',
                        Some('r') => '\r',
                        Some('t') => '\t',
                        Some('0') => '\0',
                        Some('\\') => '\\',
                        Some('"') => '"',
                        _ => {
                            return Err(ParserError::UnexpectedEscape.sinherit(&t_start));
                        }
                    });
                }
                Syntax::String(self.strings.push(res))
            }
        };

        Ok(self.syntaxes.push(value.sinherit(&t_start)))
    }

    fn scratch_push(&mut self, mark: usize) -> SyntaxRange {
        let range = self.syntaxes.push_children(&self.scratch[mark..]);
        self.scratch.truncate(mark);
        range
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
    fn from(error: SLexerError) -> Self {
        ParserError::LexerError(error.value).sinherit(&error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(source: &str) -> Result<SyntaxTree, SParserError> {
        let mut lexer = Lexer::new(source);
        Parser::new().parse(&mut lexer)
    }

    fn format_syntax_to_string(tokens: &SyntaxTree) -> String {
        format!("{:?}", tokens).trim().to_string()
    }

    #[test]
    fn test_parse_list_nested() {
        assert_eq!(
            format_syntax_to_string(&parse_str(r#"(+ 1 (* 2 3))"#).unwrap()),
            r#"List(Symbol("+"), Integer(1), List(Symbol("*"), Integer(2), Integer(3)))"#
        )
    }

    #[test]
    fn test_parse_list_quote_a() {
        assert_eq!(
            format_syntax_to_string(&parse_str(r#"'(1 2)"#).unwrap()),
            r#"List(Symbol("quote"), List(Integer(1), Integer(2)))"#
        )
    }

    #[test]
    fn test_parse_symbol() {
        assert_eq!(format_syntax_to_string(&parse_str(r#"foo"#).unwrap()), r#"Symbol("foo")"#)
    }

    #[test]
    fn test_parse_string() {
        assert_eq!(
            format_syntax_to_string(&parse_str(r#""Hello, world!""#).unwrap()),
            r#"String("Hello, world!")"#
        )
    }

    #[test]
    fn test_parse_string_escaped_a() {
        assert_eq!(
            format_syntax_to_string(&parse_str(r#""Hello, \"world\"!""#).unwrap()),
            r#"String("Hello, "world"!")"#
        )
    }

    #[test]
    fn test_parse_string_escaped_b() {
        assert_eq!(
            format_syntax_to_string(&parse_str(r#""Hello,\tworld!""#).unwrap()),
            r#"String("Hello,	world!")"#
        )
    }

    #[test]
    fn test_parse_string_escaped_c() {
        assert_eq!(
            format_syntax_to_string(&parse_str(r#""Hello, \\world!""#).unwrap()),
            r#"String("Hello, \world!")"#
        )
    }

    #[test]
    fn test_lexer_error() {
        assert_eq!(
            parse_str(r#""Hello, world!"#),
            Err(LexerError::UnterminatedString.sbetween(0, 14).into())
        );
    }

    #[test]
    fn test_parse_missing_right_paren() {
        assert_eq!(parse_str(r#"()(()"#), Err(ParserError::MissingRightParen.sbetween(2, 5)))
    }

    #[test]
    fn test_parse_unexpected_right_paren() {
        assert_eq!(parse_str(r#"()(()))"#), Err(ParserError::UnexpectedRightParen.sbetween(6, 7)))
    }

    #[test]
    fn test_parse_nothing_to_quote() {
        assert_eq!(parse_str(r#"'()'"#), Err(ParserError::NothingToQuote.sbetween(3, 4)))
    }

    #[test]
    fn test_parse_unexpected_escape() {
        assert_eq!(
            parse_str(r#""Hello, \x world!""#),
            Err(ParserError::UnexpectedEscape.sbetween(0, 18))
        )
    }
}
