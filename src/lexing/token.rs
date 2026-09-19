use {crate::span::Spanned, std::fmt::Formatter};

pub type SToken<'s> = Spanned<Token<'s>>;

#[derive(Clone, Copy, PartialEq)]
pub enum Token<'s> {
    End,
    ParenLeft,
    ParenRight,
    Quote,
    Symbol(&'s str),
    Integer(i64),
    Float(f64),
    String(&'s str),
}

impl std::fmt::Debug for Token<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::End => write!(f, "End"),
            Token::ParenLeft => write!(f, "ParenLeft"),
            Token::ParenRight => write!(f, "ParenRight"),
            Token::Quote => write!(f, "Quote"),
            Token::Symbol(v) => write!(f, "Symbol(\"{}\")", v),
            Token::Integer(v) => write!(f, "Integer({})", v),
            Token::Float(v) => write!(f, "Float({:?})", v),
            Token::String(v) => write!(f, "String(\"{}\")", v),
        }
    }
}
