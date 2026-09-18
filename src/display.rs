use crate::symbol::{SymbolId, SymbolTable, SymbolTableExt};

impl std::fmt::Display for WithDisplayContext<'_, SymbolId> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\"{}\"",
            " ".repeat(self.indent * self.indent_size.unwrap_or(0)),
            self.symbol_table
                .get_symbol(&self.value)
                .map(|s| s.to_string())
                .unwrap_or(format!("{:08x}", self.value.0))
        )
    }
}

pub struct WithDisplayContext<'a, T> {
    pub value: &'a T,
    pub symbol_table: Option<&'a SymbolTable>,
    pub indent_size: Option<usize>,
    pub indent: usize,
}

impl<'a, T> WithDisplayContext<'a, T> {
    pub fn new(value: &'a T, symbol_table: Option<&'a SymbolTable>) -> Self {
        Self { value, symbol_table, indent_size: None, indent: 0 }
    }

    pub fn set_indent(self, indent_size: usize) -> Self {
        Self { indent_size: Some(indent_size), ..self }
    }

    pub fn indent(self) -> Self {
        Self { indent: self.indent + 1, ..self }
    }

    pub fn no_indent(self) -> Self {
        Self { indent_size: None, ..self }
    }

    pub fn make_indent(&self) -> String {
        " ".repeat(self.indent * self.indent_size.unwrap_or(0))
    }
}

pub trait WithDisplayContextExt<T> {
    fn with_symbols<'a>(&'a self, symbol_table: &'a SymbolTable) -> WithDisplayContext<'a, T>;

    fn without_symbols(&'_ self) -> WithDisplayContext<'_, T>;

    fn context<'a, U>(&'a self, other: &WithDisplayContext<'a, U>) -> WithDisplayContext<'a, T> {
        let mut context = match other.symbol_table {
            Some(symbol_table) => self.with_symbols(symbol_table),
            None => self.without_symbols(),
        };
        context.indent_size = other.indent_size;
        context.indent = other.indent;
        context
    }
}

impl<T> WithDisplayContextExt<T> for T {
    fn with_symbols<'a>(&'a self, symbol_table: &'a SymbolTable) -> WithDisplayContext<'a, T> {
        WithDisplayContext::new(self, Some(symbol_table))
    }

    fn without_symbols(&'_ self) -> WithDisplayContext<'_, T> {
        WithDisplayContext::new(self, None)
    }
}
