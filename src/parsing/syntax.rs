use {
    crate::{if_not_last, span::Spanned},
    std::{collections::HashMap, fmt::Formatter, rc::Rc},
};

pub type SSyntax = Spanned<Syntax>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Syntax {
    Symbol(SymbolId),
    Integer(i64),
    Float(f64),
    String(StringId),
    List(SyntaxRange),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxRange {
    start: u32,
    len: u32,
}

impl SyntaxRange {
    pub fn new(start: u32, len: u32) -> Self {
        Self { start, len }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxStore {
    syntaxes: Vec<SSyntax>,
    children: Vec<SyntaxId>,
}

impl SyntaxStore {
    pub fn new() -> Self {
        Self { syntaxes: Vec::new(), children: Vec::new() }
    }

    pub fn get(&self, id: SyntaxId) -> &SSyntax {
        &self.syntaxes[id.0 as usize]
    }

    pub fn children(&self, range: SyntaxRange) -> &[SyntaxId] {
        let start = range.start as usize;
        &self.children[start..start + range.len as usize]
    }

    pub fn push(&mut self, node: SSyntax) -> SyntaxId {
        self.syntaxes.push(node);
        SyntaxId(self.syntaxes.len() as u32 - 1)
    }

    pub fn push_children(&mut self, children: &[SyntaxId]) -> SyntaxRange {
        let range = SyntaxRange { start: self.children.len() as u32, len: children.len() as u32 };
        self.children.extend_from_slice(children);
        range
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u32);

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolStore {
    id_to_name: Vec<Rc<str>>,
    name_to_id: HashMap<Rc<str>, SymbolId>,
}

impl SymbolStore {
    pub fn new() -> Self {
        Self { id_to_name: Vec::new(), name_to_id: HashMap::new() }
    }

    pub fn push(&mut self, name: &str) -> SymbolId {
        if let Some(&id) = self.name_to_id.get(name) {
            return id;
        }

        let id = SymbolId(u32::try_from(self.id_to_name.len()).expect("symbol limit exceeded"));
        let name: Rc<str> = Rc::from(name);

        self.id_to_name.push(Rc::clone(&name));
        self.name_to_id.insert(name, id);

        id
    }

    pub fn get(&self, id: SymbolId) -> &str {
        &self.id_to_name[id.0 as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringId(u32);

#[derive(Debug, Clone, PartialEq)]
pub struct StringStore {
    strings: Vec<String>,
}

impl StringStore {
    pub fn new() -> Self {
        Self { strings: Vec::new() }
    }

    pub fn push(&mut self, value: String) -> StringId {
        let id = StringId(u32::try_from(self.strings.len()).expect("string limit exceeded"));
        self.strings.push(value);
        id
    }

    pub fn get(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }
}

#[derive(Clone, PartialEq)]
pub struct SyntaxTree {
    pub roots: SyntaxRange,
    pub syntaxes: SyntaxStore,
    pub symbols: SymbolStore,
    pub strings: StringStore,
}

impl SyntaxTree {
    fn format_syntax(&self, syntax: SyntaxId, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.syntaxes.get(syntax).value {
            Syntax::Symbol(v) => write!(f, "Symbol(\"{}\")", self.symbols.get(v)),
            Syntax::Integer(v) => write!(f, "Integer({})", v),
            Syntax::Float(v) => write!(f, "Float({})", v),
            Syntax::String(v) => write!(f, "String(\"{}\")", self.strings.get(v)),
            Syntax::List(v) => {
                let children = self.syntaxes.children(v);
                write!(f, "List(")?;
                for (i, child) in children.iter().enumerate() {
                    self.format_syntax(*child, f)?;
                    if_not_last!(children, i, write!(f, ", ")?);
                }
                write!(f, ")")
            }
        }
    }
}

impl std::fmt::Debug for SyntaxTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for root in self.syntaxes.children(self.roots) {
            self.format_syntax(*root, f)?;
            write!(f, "\n")?;
        }
        Ok(())
    }
}
