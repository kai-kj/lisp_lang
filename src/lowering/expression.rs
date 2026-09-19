use crate::{span::Spanned, util::if_not_last};

pub type SExpression<'s> = Spanned<Expression<'s>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression<'s> {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(&'s str),
    Symbol(&'s str),
    Call {
        call: ExpressionID,
        args: Vec<ExpressionID>,
    },
    If {
        cond: ExpressionID,
        t_branch: ExpressionID,
        f_branch: ExpressionID,
    },
    Function {
        args: Vec<&'s str>,
        body: Vec<ExpressionID>,
    },
    Define {
        name: &'s str,
        value: ExpressionID,
    },
}

pub struct ExpressionTree<'s> {
    roots: Vec<ExpressionID>,
    expressions: Vec<SExpression<'s>>,
}

impl<'s> ExpressionTree<'s> {
    pub fn new() -> Self {
        Self { roots: Vec::new(), expressions: Vec::new() }
    }

    pub fn get(&self, id: ExpressionID) -> &SExpression<'s> {
        &self.expressions[id.0]
    }

    fn format_expression(
        &self,
        expression: ExpressionID,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match &self.get(expression).value {
            Expression::Null => write!(f, "Null"),
            Expression::Boolean(v) => write!(f, "Boolean({})", v),
            Expression::Integer(v) => write!(f, "Integer({})", v),
            Expression::Float(v) => write!(f, "Float({})", v),
            Expression::String(v) => write!(f, "String({})", v),
            Expression::Symbol(v) => write!(f, "Symbol({})", v),
            Expression::Call { call, args } => {
                write!(f, "Call(")?;
                self.format_expression(*call, f)?;
                write!(f, ", ")?;
                for (i, arg) in args.iter().enumerate() {
                    self.format_expression(*arg, f)?;
                    if_not_last!(args, i, write!(f, ", ")?);
                }
                write!(f, ")")
            }
            Expression::If { cond, t_branch, f_branch } => {
                write!(f, "If(")?;
                self.format_expression(*cond, f)?;
                write!(f, ", ")?;
                self.format_expression(*t_branch, f)?;
                write!(f, ", ")?;
                self.format_expression(*f_branch, f)?;
                write!(f, ")")
            }
            Expression::Function { args, body } => {
                write!(f, "Function(")?;
                for (i, arg) in args.iter().enumerate() {
                    write!(f, "{}", arg)?;
                    if_not_last!(args, i, write!(f, ", ")?);
                }
                write!(f, ", ")?;
                for (i, expr) in body.iter().enumerate() {
                    self.format_expression(*expr, f)?;
                    if_not_last!(body, i, write!(f, ", ")?);
                }
                write!(f, ")")
            }
            Expression::Define { name, value } => {
                write!(f, "Define(")?;
                write!(f, "{}", name)?;
                write!(f, ", ")?;
                self.format_expression(*value, f)?;
                write!(f, ")")
            }
        }
    }
}

impl<'s> std::fmt::Display for ExpressionTree<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for root in self.roots.iter() {
            self.format_expression(*root, f)?;
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ExpressionID(usize);
