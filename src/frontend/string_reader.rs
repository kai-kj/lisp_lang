use crate::{
    frontend::event::{Event, ReadError, Reader, SEvent, SReadError},
    span::SpannedExt,
};

pub struct StringReader<'s> {
    source: &'s str,
    pos: usize,
    next: Result<SEvent, SReadError>,
}

macro_rules! event {
    ($kind:ident $(($value:expr))?, $start:expr, $end:expr) => {
        Ok(Event::$kind $(($value))?.sbetween($start, $end))
    };
}

macro_rules! error {
    ($kind:ident, $start:expr, $end:expr) => {
        Err(ReadError::$kind.sbetween($start, $end))
    };
}

impl<'s> StringReader<'s> {
    pub fn new(source: &'s str) -> Self {
        let mut lexer = Self { source, pos: 0, next: event!(SourceEnd, 0, 1) };
        lexer.next = lexer.scan();
        lexer
    }

    fn scan(&mut self) -> Result<SEvent, SReadError> {
        self.advance_while(|_, c| c.is_whitespace());
        let start_pos = self.pos;

        match self.current() {
            Some('(') => {
                self.advance();
                event!(ListStart, start_pos, start_pos + 1)
            }
            Some(')') => {
                self.advance();
                event!(ListEnd, start_pos, start_pos + 1)
            }
            Some('\'') => {
                self.advance();
                event!(Quote, start_pos, start_pos + 1)
            }
            Some('"') => {
                let mut string = String::new();
                self.advance();
                while let Some(c) = self.current() {
                    match c {
                        '"' => {
                            self.advance();
                            return event!(String(string), start_pos, self.pos);
                        }
                        '\\' => {
                            let sequence_pos = self.pos;
                            self.advance();
                            string.push(match self.current() {
                                Some('n') => '\n',
                                Some('r') => '\r',
                                Some('t') => '\t',
                                Some('0') => '\0',
                                Some('\\') => '\\',
                                Some('"') => '"',
                                _ => {
                                    return error!(
                                        InvalidEscapeSequence,
                                        sequence_pos,
                                        self.pos + 1
                                    );
                                }
                            });
                        }
                        _ => {
                            string.push(c);
                        }
                    }
                    self.advance();
                }
                error!(UnterminatedString, start_pos, self.pos)
            }
            Some(_) => {
                self.advance_while(|_, c| {
                    !c.is_whitespace() && !matches!(c, '(' | ')' | '\'' | '"')
                });

                let characters = &self.source[start_pos..self.pos];

                if characters.starts_with('\\') {
                    return error!(InvalidSymbol, start_pos, self.pos);
                }

                let possible_number = characters
                    .bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.'));

                if possible_number {
                    if let Ok(number) = characters.parse::<i64>() {
                        return event!(Integer(number), start_pos, self.pos);
                    }
                    if let Ok(number) = characters.parse::<f64>() {
                        return event!(Float(number), start_pos, self.pos);
                    }
                }

                event!(Symbol(characters.to_string()), start_pos, self.pos)
            }
            None => event!(SourceEnd, self.source.len(), self.source.len() + 1),
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

impl<'s> Reader for StringReader<'s> {
    fn peek(&self) -> Result<&SEvent, SReadError> {
        self.next.as_ref().map_err(|err| *err)
    }

    fn next(&mut self) -> Result<SEvent, SReadError> {
        let next = self.scan();
        std::mem::replace(&mut self.next, next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! events {
        ($($kind:ident $(($value:expr))?),* $(,)?) => {
            Ok(vec![$(events!(@expr $kind $(($value))?)),*])
        };

        (@expr $kind:ident $(($value:expr))?) => {Event::$kind $(($value.into()))?};
    }

    fn read_str_to_event_vec(input: &str) -> Result<Vec<Event>, SReadError> {
        let mut reader = StringReader::new(input);
        let mut events = vec![];
        loop {
            match reader.next() {
                Err(e) => return Err(e),
                Ok(event) if event.value == Event::SourceEnd => break,
                Ok(event) => events.push(event.value),
            }
        }
        Ok(events)
    }

    #[test]
    fn test_parens() {
        assert_eq!(
            read_str_to_event_vec(r#"('())"#),
            events!(ListStart, Quote, ListStart, ListEnd, ListEnd)
        );
    }

    #[test]
    fn test_symbol() {
        assert_eq!(
            read_str_to_event_vec(r#"+ - foo bar"#),
            events!(Symbol("+"), Symbol("-"), Symbol("foo"), Symbol("bar"))
        );
    }

    #[test]
    fn test_int() {
        assert_eq!(
            read_str_to_event_vec(r#"-42 42 +42"#),
            events!(Integer(-42), Integer(42), Integer(42))
        );
    }

    #[test]
    fn test_int_zero() {
        assert_eq!(
            read_str_to_event_vec(r#"-0 0 +0"#),
            events!(Integer(0), Integer(0), Integer(0))
        );
    }

    #[test]
    fn test_float() {
        assert_eq!(
            read_str_to_event_vec(r#"-42.0 42.0 +42.0"#),
            events!(Float(-42.0), Float(42.0), Float(42.0))
        );
    }

    #[test]
    fn test_float_zero() {
        assert_eq!(
            read_str_to_event_vec(r#"-0.0 0.0 +0.0"#),
            events!(Float(-0.0), Float(0.0), Float(0.0))
        );
    }

    #[test]
    fn test_string() {
        assert_eq!(read_str_to_event_vec(r#""Hello, world!""#), events!(String("Hello, world!")));
    }

    #[test]
    fn test_string_escaped_a() {
        assert_eq!(
            read_str_to_event_vec(r#""Hello, \"world\"!""#),
            events!(String("Hello, \"world\"!"))
        )
    }

    #[test]
    fn test_string_escaped_b() {
        assert_eq!(read_str_to_event_vec(r#""Hello,\tworld!""#), events!(String("Hello,	world!")))
    }

    #[test]
    fn test_parse_string_escaped_c() {
        assert_eq!(
            read_str_to_event_vec(r#""Hello, \\world!""#),
            events!(String("Hello, \\world!"))
        )
    }

    #[test]
    fn test_unterminated_string() {
        assert_eq!(read_str_to_event_vec(r#""Hello, world!"#), error!(UnterminatedString, 0, 14));
    }

    #[test]
    fn test_unexpected_escape_sequence() {
        assert_eq!(
            read_str_to_event_vec(r#""Hello, \world!"#),
            error!(InvalidEscapeSequence, 8, 10)
        );
    }
}
