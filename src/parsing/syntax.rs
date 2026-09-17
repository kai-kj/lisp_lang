use crate::prelude::*;

pub type SSyntax = Spanned<Syntax>;

#[derive(Debug, Clone, PartialEq)]
pub enum Syntax {
    If,
    Lambda,
    Define,
    Quote,
    Null,
    Symbol(SymbolId),
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    List(Vec<SSyntax>),
}

impl<'a> std::fmt::Display for WithDisplayContext<'a, SSyntax> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.make_indent())?;
        match &self.value.value {
            Syntax::If => write!(f, "If"),
            Syntax::Lambda => write!(f, "Lambda"),
            Syntax::Define => write!(f, "Define"),
            Syntax::Quote => write!(f, "Quote"),
            Syntax::Null => write!(f, "Null"),
            Syntax::Symbol(v) => write!(f, "Symbol({})", v.context(self).no_indent()),
            Syntax::Boolean(v) => write!(f, "Boolean({})", v),
            Syntax::Integer(v) => write!(f, "Integer({})", v),
            Syntax::Float(v) => write!(f, "Float({})", v),
            Syntax::String(v) => write!(f, "String(\"{}\")", v),
            Syntax::List(v) => {
                if self.indent_size.is_some() {
                    write!(f, "List(\n")?;
                    v.iter().try_for_each(|c| writeln!(f, "{},", c.context(self).indent()))?;
                    write!(f, "{})", self.make_indent())
                } else {
                    let v = v.iter().map(|c| format!("{}", c.context(self))).collect::<Vec<_>>();
                    write!(f, "List({})", v.join(", "))
                }
            }
        }
    }
}

#[macro_export]
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

pub use syntax_list;
