#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

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

pub trait SymbolTableExt {
    fn get_symbol(&self, symbols: &SymbolId) -> Option<&str>;
}

impl SymbolTableExt for Option<&SymbolTable> {
    fn get_symbol(&self, id: &SymbolId) -> Option<&str> {
        self.as_ref().and_then(|t| t.get_symbol(id))
    }
}
