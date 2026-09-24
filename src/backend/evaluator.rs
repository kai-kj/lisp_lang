use {
    crate::{
        check,
        expression::{BindKind, Expression, ExpressionId, ExpressionRange, SymbolId},
        runtime::{
            error::{RuntimeError, SRuntimeError},
            session::Session,
            value::{BuiltinFn, UserFn, Value},
        },
        span::{ResultSpannedExt, Span, SpannedExt},
        util::Environment,
    },
    std::rc::Rc,
};

pub struct Evaluator {
    root_env: Rc<Environment<SymbolId, Value>>,
}

macro_rules! error {
    ($kind:ident, $span:expr) => {
        Err(RuntimeError::$kind.scopy($span))
    };
}

impl Evaluator {
    pub fn new(sesh: &mut Session, builtins: Vec<(&'static str, BuiltinFn)>) -> Self {
        let root_env = Environment::new();

        for (name, builtin) in builtins {
            let name = sesh.push_symbol(name);
            root_env.def(name, Value::BuiltinFn(Rc::new(builtin))).unwrap();
        }

        Self { root_env }
    }

    pub fn get_root_env(&self) -> &Rc<Environment<SymbolId, Value>> {
        &self.root_env
    }

    pub fn evaluate(
        &self,
        sesh: &mut Session,
        expressions: ExpressionRange,
    ) -> Result<Value, SRuntimeError> {
        let mut result = Value::Null;
        for expression in sesh.get_expressions(expressions).to_owned() {
            result = self.evaluate_expr(sesh, &self.root_env, expression)?;
        }
        Ok(result)
    }

    pub fn evaluate_macro(
        &self,
        sesh: &mut Session,
        f: &UserFn,
        args: &[ExpressionId],
        p_span: Span,
    ) -> Result<Value, SRuntimeError> {
        let args = args
            .iter()
            .map(|&id| self.evaluate_expr(sesh, &self.root_env, id))
            .collect::<Result<Vec<_>, _>>()?;
        self.evaluate_user_fn(sesh, f, args, p_span)
    }

    fn evaluate_expr(
        &self,
        sesh: &mut Session,
        env: &Rc<Environment<SymbolId, Value>>,
        expr: ExpressionId,
    ) -> Result<Value, SRuntimeError> {
        let expr = *sesh.get_expression(expr);

        match expr.value {
            Expression::Null => Ok(Value::Null),
            Expression::Boolean(v) => Ok(Value::Boolean(v)),
            Expression::Integer(v) => Ok(Value::Integer(v)),
            Expression::Float(v) => Ok(Value::Float(v)),
            Expression::String(v) => Ok(Value::String(Rc::new(sesh.get_string(v).to_string()))),
            Expression::Symbol(v) => env.get(v).err_scopy(expr.span),
            Expression::SymbolQ(v) => Ok(Value::Symbol(v)),
            Expression::List(items) => self.evaluate_list(sesh, env, items),
            Expression::Do(body) => {
                let mut result = Value::Null;
                for expression in sesh.get_expressions(body).to_owned() {
                    result = self.evaluate_expr(sesh, env, expression)?;
                }
                Ok(result)
            }
            Expression::Call { call, args } => self.evaluate_call(sesh, env, call, args, expr.span),
            Expression::If { cond, t_branch, f_branch } => {
                if self.evaluate_expr(sesh, env, cond)?.is_truthy() {
                    self.evaluate_expr(sesh, env, t_branch)
                } else {
                    self.evaluate_expr(sesh, env, f_branch)
                }
            }
            Expression::Fn { req_params, rest_param, body } => Ok(Value::UserFn(Rc::new(UserFn {
                req_params,
                rest_param,
                body,
                env: env.clone(),
            }))),
            Expression::Bind { kind: BindKind::Def, name, value } => {
                let value = self.evaluate_expr(sesh, env, value)?;
                env.def(name, value).err_scopy(expr.span)?;
                Ok(Value::Null)
            }
            Expression::Bind { kind: BindKind::Set, name, value } => {
                let value = self.evaluate_expr(sesh, env, value)?;
                env.set(name, value).err_scopy(expr.span)?;
                Ok(Value::Null)
            }
        }
    }

    fn evaluate_list(
        &self,
        sesh: &mut Session,
        env: &Rc<Environment<SymbolId, Value>>,
        values: ExpressionRange,
    ) -> Result<Value, SRuntimeError> {
        let items = sesh.get_expressions(values).to_owned();
        if items.is_empty() {
            Ok(Value::Null)
        } else {
            let items = items
                .iter()
                .map(|e| self.evaluate_expr(sesh, env, *e))
                .collect::<Result<Vec<Value>, SRuntimeError>>()?;
            Ok(Value::list(items))
        }
    }

    fn evaluate_call(
        &self,
        sesh: &mut Session,
        env: &Rc<Environment<SymbolId, Value>>,
        call: ExpressionId,
        args: ExpressionRange,
        p_span: Span,
    ) -> Result<Value, SRuntimeError> {
        let call = self.evaluate_expr(sesh, env, call)?;
        let args = sesh
            .get_expressions(args)
            .to_owned()
            .into_iter()
            .map(|arg| self.evaluate_expr(sesh, env, arg))
            .collect::<Result<Vec<_>, _>>()?;

        match call {
            Value::BuiltinFn(f) => {
                check!(f.accepts(args.len()), error!(UnexpectedParamCount, p_span));
                (f.body)(sesh, &args).err_scopy(p_span)
            }
            Value::UserFn(f) => self.evaluate_user_fn(sesh, &f, args, p_span),
            _ => Err(RuntimeError::NotAFunction.scopy(p_span)),
        }
    }

    fn evaluate_user_fn(
        &self,
        session: &mut Session,
        f: &UserFn,
        args: Vec<Value>,
        p_span: Span,
    ) -> Result<Value, SRuntimeError> {
        check!(f.accepts(args.len()), error!(UnexpectedParamCount, p_span));

        let fn_env = f.env.child();
        let mut args = args.into_iter();

        for name in session.get_params(f.req_params).to_vec() {
            let value = args.next().unwrap();
            fn_env.def(name, value).err_scopy(p_span)?;
        }

        if let Some(name) = f.rest_param {
            fn_env.def(name, Value::list(args.collect())).err_scopy(p_span)?;
        }

        self.evaluate_expr(session, &fn_env, f.body)
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
        let mut sesh = Session::new();
        let roots =
            Parser::new(&mut sesh, None).parse(&mut sesh, &mut StringReader::new(source))?;
        let evaluator = Evaluator::new(&mut sesh, make_builtins());
        evaluator.evaluate(&mut sesh, roots)
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
    fn test_call_builtin_a() {
        assert_eq!(eval_str_to_value("(+ 1 2)").unwrap(), Value::Integer(3))
    }

    #[test]
    fn test_call_builtin_b() {
        assert_eq!(eval_str_to_value("(+ 1 2 3)").unwrap(), Value::Integer(6))
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
            Value::list(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)])
        )
    }
}
