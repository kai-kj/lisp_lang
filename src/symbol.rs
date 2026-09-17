#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(usize);

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
    fn with_symbols<'a>(&'a self, symbol_table: &'a SymbolTable) -> WithDisplayContext<T>;

    fn without_symbols(&self) -> WithDisplayContext<T>;

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

    fn without_symbols(&self) -> WithDisplayContext<T> {
        WithDisplayContext::new(self, None)
    }
}

pub struct SymbolTable {
    id_to_name: Vec<std::rc::Rc<str>>,
    name_to_id: std::collections::HashMap<std::rc::Rc<str>, SymbolId>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self { id_to_name: Vec::new(), name_to_id: std::collections::HashMap::new() }
    }

    pub fn add_symbol(&mut self, name: &str) -> SymbolId {
        if let Some(&id) = self.name_to_id.get(name) {
            return id;
        }

        let id = SymbolId(self.id_to_name.len());
        let name: std::rc::Rc<str> = name.into();

        self.id_to_name.push(name.clone());
        self.name_to_id.insert(name, id);

        id
    }

    pub fn get_symbol(&self, id: &SymbolId) -> Option<&str> {
        self.id_to_name.get(id.0).map(|s| s.as_ref())
    }
}

pub trait SymbolTableOptionExt {
    fn get_symbol(&self, symbols: &SymbolId) -> Option<&str>;
}

impl SymbolTableOptionExt for Option<&SymbolTable> {
    fn get_symbol(&self, id: &SymbolId) -> Option<&str> {
        self.as_ref().and_then(|t| t.get_symbol(id))
    }
}
