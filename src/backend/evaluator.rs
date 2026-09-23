use {
    crate::{
        check,
        expression::{BindKind, Expression, ExpressionId, ExpressionRange},
        runtime::{
            environment::Environment,
            error::{RuntimeError, SRuntimeError},
            session::Session,
            value::{BuiltinFn, UserFn, Value},
        },
        span::{ResultSpannedExt, Span, SpannedExt},
    },
    std::rc::Rc,
};

pub struct Evaluator {
    root_env: Rc<Environment>,
}

macro_rules! error {
    ($kind:ident, $span:expr) => {
        Err(RuntimeError::$kind.scopy($span))
    };
}

impl Evaluator {
    pub fn new(session: &mut Session, builtins: Vec<(&'static str, BuiltinFn)>) -> Self {
        let root_env = Environment::new();

        for (name, builtin) in builtins {
            let name = session.push_symbol(name);
            root_env.def(name, Value::BuiltinFn(Rc::new(builtin))).unwrap();
        }

        Self { root_env }
    }

    pub fn get_root_env(&self) -> &Rc<Environment> {
        &self.root_env
    }

    pub fn evaluate(
        &self,
        session: &mut Session,
        expressions: ExpressionRange,
    ) -> Result<Value, SRuntimeError> {
        let mut result = Value::Null;
        for expression in session.get_expressions(expressions).to_owned() {
            result = self.evaluate_expr(session, &self.root_env, expression)?;
        }
        Ok(result)
    }

    pub fn evaluate_macro(
        &self,
        session: &mut Session,
        f: &UserFn,
        args: &[ExpressionId],
        p_span: Span,
    ) -> Result<Value, SRuntimeError> {
        let params = session.get_params(f.params).to_vec();
        let fn_env = f.env.child();

        check!(args.len() == params.len(), error!(UnexpectedParamCount, p_span));
        for (name, value) in params.into_iter().zip(args.into_iter()) {
            let value = self.evaluate_expr(session, &fn_env, *value)?;
            fn_env.def(name, value).err_scopy(p_span)?;
        }

        self.evaluate_expr(session, &fn_env, f.body)
    }

    fn evaluate_expr(
        &self,
        session: &mut Session,
        env: &Rc<Environment>,
        expr: ExpressionId,
    ) -> Result<Value, SRuntimeError> {
        let expr = *session.get_expression(expr);

        match expr.value {
            Expression::Null => Ok(Value::Null),
            Expression::Boolean(v) => Ok(Value::Boolean(v)),
            Expression::Integer(v) => Ok(Value::Integer(v)),
            Expression::Float(v) => Ok(Value::Float(v)),
            Expression::String(v) => Ok(Value::String(Rc::new(session.get_string(v).to_string()))),
            Expression::Symbol(v) => env.get(v).err_scopy(expr.span),
            Expression::SymbolQ(v) => Ok(Value::Symbol(v)),
            Expression::List(items) => self.evaluate_list(session, env, items),
            Expression::Do(body) => {
                let mut result = Value::Null;
                for expression in session.get_expressions(body).to_owned() {
                    result = self.evaluate_expr(session, env, expression)?;
                }
                Ok(result)
            }
            Expression::Call { call, args } => {
                self.evaluate_call(session, env, call, args, expr.span)
            }
            Expression::If { cond, t_branch, f_branch } => {
                if self.evaluate_expr(session, env, cond)?.is_truthy() {
                    self.evaluate_expr(session, env, t_branch)
                } else {
                    self.evaluate_expr(session, env, f_branch)
                }
            }
            Expression::Fn { params, body } => {
                Ok(Value::UserFn(Rc::new(UserFn { params, body, env: env.clone() })))
            }
            Expression::Bind { kind: BindKind::Def, name, value } => {
                let value = self.evaluate_expr(session, env, value)?;
                env.def(name, value).err_scopy(expr.span)?;
                Ok(Value::Null)
            }
            Expression::Bind { kind: BindKind::Set, name, value } => {
                let value = self.evaluate_expr(session, env, value)?;
                env.set(name, value).err_scopy(expr.span)?;
                Ok(Value::Null)
            }
        }
    }

    fn evaluate_list(
        &self,
        session: &mut Session,
        env: &Rc<Environment>,
        values: ExpressionRange,
    ) -> Result<Value, SRuntimeError> {
        let items = session.get_expressions(values).to_owned();
        if items.is_empty() {
            Ok(Value::Null)
        } else {
            let items = items
                .iter()
                .map(|e| self.evaluate_expr(session, env, *e))
                .collect::<Result<Vec<Value>, SRuntimeError>>()?;
            Ok(Value::List(Rc::new(items)))
        }
    }

    fn evaluate_call(
        &self,
        session: &mut Session,
        env: &Rc<Environment>,
        call: ExpressionId,
        args: ExpressionRange,
        p_span: Span,
    ) -> Result<Value, SRuntimeError> {
        let call = self.evaluate_expr(session, env, call)?;
        let args = session
            .get_expressions(args)
            .to_owned()
            .into_iter()
            .map(|arg| self.evaluate_expr(session, env, arg))
            .collect::<Result<Vec<_>, _>>()?;

        match call {
            Value::BuiltinFn(f) => {
                check!(args.len() == f.params.len(), error!(UnexpectedParamCount, p_span));
                (f.body)(session, &args).err_scopy(p_span)
            }
            Value::UserFn(f) => {
                let params = session.get_params(f.params).to_vec();
                let fn_env = f.env.child();

                check!(args.len() == params.len(), error!(UnexpectedParamCount, p_span));
                for (name, value) in params.into_iter().zip(args.into_iter()) {
                    fn_env.def(name, value).err_scopy(p_span)?;
                }

                self.evaluate_expr(session, &fn_env, f.body)
            }
            _ => Err(RuntimeError::NotAFunction.scopy(p_span)),
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            frontend::{parser::Parser, string_reader::StringReader},
            runtime::builtins::make_builtins,
        },
    };

    fn eval_str_to_value(source: &str) -> Result<Value, SRuntimeError> {
        let mut session = Session::new();
        let roots =
            Parser::new(&mut session).parse(&mut session, &mut StringReader::new(source))?;
        let evaluator = Evaluator::new(&mut session, make_builtins());
        evaluator.evaluate(&mut session, roots)
    }

    #[test]
    fn test_literal() {
        assert_eq!(eval_str_to_value("1").unwrap(), Value::Integer(1))
    }

    #[test]
    fn test_do() {
        assert_eq!(eval_str_to_value("(do 1 2)").unwrap(), Value::Integer(2))
    }

    #[test]
    fn test_call_builtin() {
        assert_eq!(eval_str_to_value("(+ 1 2)").unwrap(), Value::Integer(3))
    }

    #[test]
    fn test_call_user() {
        assert_eq!(eval_str_to_value("((fn (a b) (+ a b)) 1 2)").unwrap(), Value::Integer(3))
    }

    #[test]
    fn test_if_a() {
        assert_eq!(eval_str_to_value("(if true 4 2)").unwrap(), Value::Integer(4))
    }

    #[test]
    fn test_if_b() {
        assert_eq!(eval_str_to_value("(if false 4 2)").unwrap(), Value::Integer(2))
    }

    #[test]
    fn test_define_a() {
        assert_eq!(
            eval_str_to_value("(def add (fn (a b) (+ a b))) (add 1 2)").unwrap(),
            Value::Integer(3)
        )
    }

    #[test]
    fn test_define_b() {
        assert_eq!(
            eval_str_to_value(
                r#"
                (def make_counter (fn () (do
                    (def c 0)
                    (fn () (do (set c (+ c 1)) c)))))
                (def counter (make_counter))
                (counter) (counter) (counter)
                "#
            )
            .unwrap(),
            Value::Integer(3)
        )
    }

    #[test]
    fn test_quote() {
        assert_eq!(
            eval_str_to_value("'(1 2 3)").unwrap(),
            Value::List(Rc::new(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]))
        )
    }
}
