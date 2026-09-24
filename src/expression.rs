use {
    crate::{
        runtime::session::ParamRange,
        span::Spanned,
        util::{ArenaId, ArenaRange},
    },
    std::{collections::HashMap, rc::Rc},
};

pub type SExpression = Spanned<Expression>;
pub type ExpressionId = ArenaId<SExpression>;
pub type ExpressionRange = ArenaRange<SExpression>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Expression {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(StringRange),
    Symbol(SymbolId),
    SymbolQ(SymbolId),
    List(ExpressionRange),
    Do(ExpressionRange),
    Call { call: ExpressionId, args: ExpressionRange },
    If { cond: ExpressionId, t_branch: ExpressionId, f_branch: ExpressionId },
    Fn { req_params: ParamRange, rest_param: Option<SymbolId>, body: ExpressionId },

    Bind { kind: BindKind, name: SymbolId, value: ExpressionId },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BindKind {
    Def,
    Set,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolTable {
    id_to_name: Vec<Rc<str>>,
    name_to_id: HashMap<Rc<str>, SymbolId>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self { id_to_name: Vec::new(), name_to_id: HashMap::new() }
    }

    pub fn push(&mut self, name: &str) -> SymbolId {
        if let Some(&id) = self.name_to_id.get(name) {
            return id;
        }

        let id = SymbolId(self.id_to_name.len());
        let name: Rc<str> = Rc::from(name);

        self.id_to_name.push(Rc::clone(&name));
        self.name_to_id.insert(name, id);

        id
    }

    pub fn get(&self, id: SymbolId) -> &str {
        &self.id_to_name[id.0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(usize);

#[cfg(test)]
use crate::runtime::session::Session;
use crate::runtime::session::StringRange;

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum OwnedExpression {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Symbol(String),
    SymbolQ(String),
    List(Vec<OwnedExpression>),
    Do(Vec<OwnedExpression>),
    Call {
        call: Box<OwnedExpression>,
        args: Vec<OwnedExpression>,
    },
    If {
        cond: Box<OwnedExpression>,
        t_branch: Box<OwnedExpression>,
        f_branch: Box<OwnedExpression>,
    },
    Fn {
        req_params: Vec<String>,
        rest_param: Option<String>,
        body: Box<OwnedExpression>,
    },
    Bind {
        kind: BindKind,
        name: String,
        value: Box<OwnedExpression>,
    },
}

#[cfg(test)]
impl ExpressionId {
    #[cfg(test)]
    pub fn to_owned(self, sesh: &Session) -> OwnedExpression {
        match sesh.get_expression(self).value {
            Expression::Null => OwnedExpression::Null,
            Expression::Boolean(v) => OwnedExpression::Boolean(v),
            Expression::Integer(v) => OwnedExpression::Integer(v),
            Expression::Float(v) => OwnedExpression::Float(v),
            Expression::String(v) => OwnedExpression::String(sesh.get_string(v).to_string()),
            Expression::Symbol(v) => OwnedExpression::Symbol(sesh.get_symbol(v).to_string()),
            Expression::SymbolQ(v) => OwnedExpression::SymbolQ(sesh.get_symbol(v).to_string()),
            Expression::List(v) => OwnedExpression::List(
                sesh.get_expressions(v).iter().map(|id| (*id).to_owned(sesh)).collect(),
            ),
            Expression::Do(body) => OwnedExpression::Do(
                sesh.get_expressions(body).iter().map(|id| (*id).to_owned(sesh)).collect(),
            ),
            Expression::Call { call, args } => OwnedExpression::Call {
                call: Box::new(call.to_owned(sesh)),
                args: sesh.get_expressions(args).iter().map(|id| (*id).to_owned(sesh)).collect(),
            },
            Expression::If { cond, t_branch, f_branch } => OwnedExpression::If {
                cond: Box::new(cond.to_owned(sesh)),
                t_branch: Box::new(t_branch.to_owned(sesh)),
                f_branch: Box::new(f_branch.to_owned(sesh)),
            },
            Expression::Fn { req_params, rest_param, body } => OwnedExpression::Fn {
                req_params: sesh
                    .get_params(req_params)
                    .iter()
                    .map(|id| sesh.get_symbol(*id).to_string())
                    .collect(),
                rest_param: rest_param.map(|id| sesh.get_symbol(id).to_string()),
                body: Box::new(body.to_owned(sesh)),
            },
            Expression::Bind { kind, name, value } => OwnedExpression::Bind {
                kind,
                name: sesh.get_symbol(name).to_string(),
                value: Box::new(value.to_owned(sesh)),
            },
        }
    }
}
