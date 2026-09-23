use crate::{
    expression::{ExpressionId, ExpressionRange, SExpression, SymbolId, SymbolTable},
    util::{Arena, ArenaId},
};

pub struct Session {
    expressions: Arena<SExpression>,
    symbols: SymbolTable,
    params: Vec<SymbolId>,
    strings: String,
}

impl Session {
    pub fn new() -> Self {
        Session {
            expressions: Arena::new(),
            symbols: SymbolTable::new(),
            params: Vec::new(),
            strings: String::new(),
        }
    }

    pub fn get_expression(&self, id: ExpressionId) -> &SExpression {
        self.expressions.get(id)
    }

    pub fn get_expressions(&self, range: ExpressionRange) -> &[ArenaId<SExpression>] {
        self.expressions.get_children(range)
    }

    pub fn push_expression(&mut self, expression: SExpression) -> ExpressionId {
        self.expressions.push(expression)
    }

    pub fn push_expressions(&mut self, expressions: &[ExpressionId]) -> ExpressionRange {
        self.expressions.push_children(expressions)
    }

    pub fn get_symbol(&self, id: SymbolId) -> &str {
        self.symbols.get(id)
    }

    pub fn push_symbol(&mut self, symbol: &str) -> SymbolId {
        self.symbols.push(symbol)
    }

    pub fn get_n_params(&self) -> usize {
        self.params.len()
    }

    pub fn get_params(&self, range: ParamRange) -> &[SymbolId] {
        &self.params[range.0..range.1]
    }

    pub fn push_param(&mut self, param: SymbolId) {
        self.params.push(param);
    }

    pub fn get_string(&self, range: StringRange) -> &str {
        &self.strings[range.0..range.1]
    }

    pub fn push_string(&mut self, value: &str) -> StringRange {
        let start = self.strings.len();
        self.strings.push_str(value);
        StringRange(start, self.strings.len())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StringRange(usize, usize);

impl StringRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self(start, end)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamRange(usize, usize);

impl ParamRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self(start, end)
    }
}
