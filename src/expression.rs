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
pub type StringId = ArenaId<String>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Expression {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(StringId),
    Symbol(SymbolId),
    Do(ExpressionRange),
    Call {
        call: ExpressionId,
        args: ExpressionRange,
    },
    If {
        cond: ExpressionId,
        t_branch: ExpressionId,
        f_branch: ExpressionId,
    },
    Fn {
        params: ParamRange,
        body: ExpressionId,
    },
    Def {
        name: SymbolId,
        value: ExpressionId,
    },
    Set {
        name: SymbolId,
        value: ExpressionId,
    },
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

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum OwnedExpression {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Symbol(String),
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
        args: Vec<String>,
        body: Box<OwnedExpression>,
    },
    Def {
        name: String,
        value: Box<OwnedExpression>,
    },
    Set {
        name: String,
        value: Box<OwnedExpression>,
    },
}

#[cfg(test)]
impl ExpressionId {
    #[cfg(test)]
    pub fn to_owned(self, session: &Session) -> OwnedExpression {
        match session.get_expression(self).value {
            Expression::Null => OwnedExpression::Null,
            Expression::Boolean(v) => OwnedExpression::Boolean(v),
            Expression::Integer(v) => OwnedExpression::Integer(v),
            Expression::Float(v) => OwnedExpression::Float(v),
            Expression::String(v) => OwnedExpression::String(session.get_string(v).to_string()),
            Expression::Symbol(v) => OwnedExpression::Symbol(session.get_symbol(v).to_string()),
            Expression::Do(body) => OwnedExpression::Do(
                session.get_expressions(body).iter().map(|id| (*id).to_owned(session)).collect(),
            ),
            Expression::Call { call, args } => OwnedExpression::Call {
                call: Box::new(call.to_owned(session)),
                args: session
                    .get_expressions(args)
                    .iter()
                    .map(|id| (*id).to_owned(session))
                    .collect(),
            },
            Expression::If { cond, t_branch, f_branch } => OwnedExpression::If {
                cond: Box::new(cond.to_owned(session)),
                t_branch: Box::new(t_branch.to_owned(session)),
                f_branch: Box::new(f_branch.to_owned(session)),
            },
            Expression::Fn { params, body } => OwnedExpression::Fn {
                args: session
                    .get_params(params)
                    .iter()
                    .map(|id| session.get_symbol(*id).to_string())
                    .collect(),
                body: Box::new(body.to_owned(session)),
            },
            Expression::Def { name, value } => OwnedExpression::Def {
                name: session.get_symbol(name).to_string(),
                value: Box::new(value.to_owned(session)),
            },
            Expression::Set { name, value } => OwnedExpression::Set {
                name: session.get_symbol(name).to_string(),
                value: Box::new(value.to_owned(session)),
            },
        }
    }
}
