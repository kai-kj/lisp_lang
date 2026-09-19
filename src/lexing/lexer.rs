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
        Ok(Token::$kind $(($value))?.sbetween($start, $end))
    };
}

impl<'s> Lexer<'s> {
    pub fn new(source: &'s str) -> Self {
        let mut lexer = Self { source, pos: 0, next: Ok(Token::End.sbetween(0, 1)) };
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
                Err(LexerError::UnterminatedString.sbetween(start_pos, self.pos))
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
            None => Ok(Token::End.sinherit(&self.next?)),
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
    use super::*;

    fn lex_str_to_vec(source: &str) -> Result<Vec<SToken>, SLexerError> {
        let mut lexer = Lexer::new(&source);
        let mut tokens = vec![];
        loop {
            match lexer.next() {
                Ok(token) if token.value == Token::End => break,
                Ok(token) => tokens.push(token),
                Err(err) => return Err(err),
            }
        }
        Ok(tokens)
    }

    fn format_vec_to_string(tokens: &[SToken]) -> String {
        tokens
            .iter()
            .map(|token| format!("{:?}", token.value))
            .collect::<Vec<String>>()
            .join(", ")
            .trim()
            .to_string()
    }

    #[test]
    fn test_parens() {
        assert_eq!(
            format_vec_to_string(&lex_str_to_vec(r#"('())"#).unwrap()),
            r#"ParenLeft, Quote, ParenLeft, ParenRight, ParenRight"#,
        );
    }

    #[test]
    fn test_symbol() {
        assert_eq!(
            format_vec_to_string(&lex_str_to_vec(r#"+ - foo bar"#).unwrap()),
            r#"Symbol("+"), Symbol("-"), Symbol("foo"), Symbol("bar")"#
        );
    }

    #[test]
    fn test_int() {
        assert_eq!(
            format_vec_to_string(&lex_str_to_vec(r#"-42 42 +42"#).unwrap()),
            r#"Integer(-42), Integer(42), Integer(42)"#
        );
    }

    #[test]
    fn test_int_zero() {
        assert_eq!(
            format_vec_to_string(&lex_str_to_vec(r#"-0 0 +0"#).unwrap()),
            r#"Integer(0), Integer(0), Integer(0)"#
        );
    }

    #[test]
    fn test_float() {
        assert_eq!(
            format_vec_to_string(&lex_str_to_vec(r#"-42.0 42.0 +42.0"#).unwrap()),
            r#"Float(-42.0), Float(42.0), Float(42.0)"#
        );
    }

    #[test]
    fn test_float_zero() {
        assert_eq!(
            format_vec_to_string(&lex_str_to_vec(r#"-0.0 0.0 +0.0"#).unwrap()),
            r#"Float(-0.0), Float(0.0), Float(0.0)"#
        );
    }

    #[test]
    fn test_string() {
        assert_eq!(
            format_vec_to_string(&lex_str_to_vec(r#""Hello, world!""#).unwrap()),
            r#"String("Hello, world!")"#
        );
    }

    #[test]
    fn test_string_escape() {
        assert_eq!(
            format_vec_to_string(&lex_str_to_vec(r#""Hello, \"world\"!""#).unwrap()),
            r#"String("Hello, \"world\"!")"#
        );
    }

    #[test]
    fn test_string_unterminated() {
        assert_eq!(
            lex_str_to_vec(r#""Hello, world!"#),
            Err(LexerError::UnterminatedString.sbetween(0, 14))
        );
    }
}
