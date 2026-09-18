use crate::{
    lexing::token::{SToken, Token},
    span::{Spanned, SpannedExt},
};

pub struct Lexer<'s> {
    source: &'s str,
    pos: usize,
    next: Result<SToken<'s>, SLexerError>,
}

macro_rules! make_token {
    ($start:expr, $end:expr, $kind:ident $(($value:expr))? ) => {
        Ok(Token::$kind $(($value))?.span_between($start, $end))
    };
}

impl<'s> Lexer<'s> {
    pub fn new(source: &'s str) -> Self {
        let mut lexer = Self { source, pos: 0, next: Ok(Token::End.span_none()) };
        lexer.next = lexer.scan();
        lexer
    }

    pub fn peek(&self) -> Result<SToken<'s>, SLexerError> {
        self.next
    }

    pub fn next(&mut self) -> Result<SToken<'s>, SLexerError> {
        let next = self.scan();
        std::mem::replace(&mut self.next, next)
    }

    fn scan(&mut self) -> Result<SToken<'s>, SLexerError> {
        self.advance_while(|_, c| c.is_whitespace());
        let start_pos = self.pos;

        match self.current() {
            Some('(') => {
                self.advance();
                make_token!(start_pos, start_pos + 1, ParenLeft)
            }
            Some(')') => {
                self.advance();
                make_token!(start_pos, start_pos + 1, ParenRight)
            }
            Some('\'') => {
                self.advance();
                make_token!(start_pos, start_pos + 1, Quote)
            }
            Some('"') => {
                self.advance();
                while let Some(c) = self.current() {
                    match c {
                        '"' => {
                            self.advance();
                            let string = &self.source[start_pos + 1..self.pos - 1];
                            return make_token!(start_pos, self.pos, String(string));
                        }
                        '\\' => {
                            self.advance();
                            self.advance();
                        }
                        _ => self.advance(),
                    }
                }
                Err(LexerError::UnterminatedString.span_between(start_pos, self.pos))
            }
            Some(_) => {
                self.advance_while(|_, c| {
                    !c.is_whitespace() && !matches!(c, '(' | ')' | '\'' | '"')
                });

                let characters = &self.source[start_pos..self.pos];

                let possible_number = characters
                    .bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.'));

                if possible_number {
                    if let Ok(number) = characters.parse() {
                        return make_token!(start_pos, self.pos, Integer(number));
                    }

                    if let Ok(number) = characters.parse::<f64>()
                        && number.is_finite()
                    {
                        return make_token!(start_pos, self.pos, Float(number));
                    }
                }

                make_token!(start_pos, self.pos, Symbol(characters))
            }
            None => Ok(Token::End.span(self.next?.span)),
        }
    }

    fn current(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn previous(&self) -> Option<char> {
        self.source[..self.pos].chars().next_back()
    }

    fn advance(&mut self) {
        if let Some(c) = self.current() {
            self.pos += c.len_utf8();
        }
    }

    fn advance_while(&mut self, condition: fn(Option<char>, char) -> bool) {
        while let Some(curr) = self.current() {
            if !condition(self.previous(), curr) {
                break;
            }
            self.advance();
        }
    }
}

pub type SLexerError = Spanned<LexerError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LexerError {
    UnterminatedString,
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::lexing::owned_token::{OwnedToken, SOwnedToken},
    };

    macro_rules! token_list {
        ($($kind:ident $(($value:expr))?),* $(,)?) => { Ok(vec![$(token_list!(@token $kind $(($value))?)),*]) };
        (@token String($value:expr)) => { OwnedToken::String(($value).into()).span_none() };
        (@token Symbol($value:expr)) => { OwnedToken::Symbol(($value).into()).span_none() };
        (@token $kind:ident $(($value:expr))?) => { OwnedToken::$kind $(($value))?.span_none() };
    }

    fn lex_str_to_owned_vec(source: &str) -> Result<Vec<SOwnedToken>, SLexerError> {
        let mut lexer = Lexer::new(&source);
        let mut tokens = vec![];
        loop {
            match lexer.next() {
                Ok(token) if token.value == Token::End => {
                    break;
                }
                Ok(token) => {
                    tokens.push(token.to_owned_token());
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        Ok(tokens)
    }

    #[test]
    fn test_parens() {
        assert_eq!(
            lex_str_to_owned_vec(r#"('())"#),
            token_list!(ParenLeft, Quote, ParenLeft, ParenRight, ParenRight),
        );
    }

    #[test]
    fn test_symbol() {
        assert_eq!(
            lex_str_to_owned_vec(r#"+ - foo bar"#),
            token_list!(Symbol(r#"+"#), Symbol(r#"-"#), Symbol(r#"foo"#), Symbol(r#"bar"#))
        );
    }

    #[test]
    fn test_int() {
        assert_eq!(
            lex_str_to_owned_vec(r#"-42 42 +42"#),
            token_list!(Integer(-42), Integer(42), Integer(42))
        );
    }

    #[test]
    fn test_int_zero() {
        assert_eq!(
            lex_str_to_owned_vec(r#"-0 0 +0"#),
            token_list!(Integer(0), Integer(0), Integer(0))
        );
    }

    #[test]
    fn test_float() {
        assert_eq!(
            lex_str_to_owned_vec(r#"-42.0 42.0 +42.0"#),
            token_list!(Float(-42.0), Float(42.0), Float(42.0))
        );
    }

    #[test]
    fn test_float_zero() {
        assert_eq!(
            lex_str_to_owned_vec(r#"-0.0 0.0 +0.0"#),
            token_list!(Float(0.0), Float(0.0), Float(0.0))
        );
    }

    #[test]
    fn test_string() {
        assert_eq!(
            lex_str_to_owned_vec(r#""Hello, world!""#),
            token_list!(String(r#"Hello, world!"#))
        );
    }

    #[test]
    fn test_string_escape() {
        assert_eq!(
            lex_str_to_owned_vec(r#""Hello, \"world\"!""#),
            token_list!(String(r#"Hello, \"world\"!"#))
        );
    }

    #[test]
    fn test_string_unterminated() {
        assert_eq!(
            lex_str_to_owned_vec(r#""Hello, world!"#),
            Err(LexerError::UnterminatedString.span_between(0, 14))
        );
    }
}
