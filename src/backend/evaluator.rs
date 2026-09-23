use {
    crate::{
        expression::{Expression, ExpressionId, ExpressionRange},
        runtime::{
            environment::Environment,
            error::{RuntimeError, SRuntimeError},
            session::Session,
            value::{BuiltinFunction, UserFunction, Value},
        },
        span::{ResultSpannedExt, SpannedExt},
    },
    std::rc::Rc,
};

pub struct Evaluator {
    root_env: Rc<Environment>,
}

impl Evaluator {
    pub fn new(
        session: &mut Session,
        builtins: Vec<(&'static str, BuiltinFunction)>,
    ) -> Result<Self, RuntimeError> {
        let root_env = Environment::new();

        for (name, builtin) in builtins {
            let name = session.push_symbol(name);
            root_env.def(name, Value::BuiltinFunction(Rc::new(builtin)))?;
        }

        Ok(Self { root_env })
    }

    pub fn evaluate(
        &self,
        session: &mut Session,
        expressions: ExpressionRange,
    ) -> Result<Value, SRuntimeError> {
        let mut result = Value::Null;
        for expression in session.get_expressions(expressions).to_owned() {
            result = self.evaluate_expression(session, &self.root_env, expression)?;
        }
        Ok(result)
    }

    fn evaluate_expression(
        &self,
        session: &mut Session,
        env: &Rc<Environment>,
        expression: ExpressionId,
    ) -> Result<Value, SRuntimeError> {
        let expression = *session.get_expression(expression);

        match expression.value {
            Expression::Null => Ok(Value::Null),
            Expression::Boolean(v) => Ok(Value::Boolean(v)),
            Expression::Integer(v) => Ok(Value::Integer(v)),
            Expression::Float(v) => Ok(Value::Float(v)),
            Expression::String(v) => Ok(Value::String(Rc::new(session.get_string(v).to_string()))),
            Expression::Symbol(v) => env.get(v).err_scopy(expression.span),
            Expression::List(v) => {
                let items = session.get_expressions(v).to_owned();
                if items.is_empty() {
                    Ok(Value::Null)
                } else {
                    let items = items
                        .iter()
                        .map(|e| self.evaluate_expression(session, env, *e))
                        .collect::<Result<Vec<Value>, SRuntimeError>>()?;
                    Ok(Value::List(Rc::new(items)))
                }
            }
            Expression::Do(expressions) => {
                let mut result = Value::Null;
                let expression = session.get_expressions(expressions).to_owned();
                for expression in expression {
                    result = self.evaluate_expression(session, env, expression)?;
                }
                Ok(result)
            }
            Expression::Call { call, args } => {
                let call = self.evaluate_expression(session, env, call)?;

                let args = session.get_expressions(args).to_owned();

                let args = args
                    .into_iter()
                    .map(|arg| self.evaluate_expression(session, env, arg))
                    .collect::<Result<Vec<_>, _>>()?;

                match call {
                    Value::BuiltinFunction(function) => {
                        if args.len() != function.params.len() {
                            return Err(RuntimeError::UnexpectedParamCount.scopy(expression.span));
                        }
                        (function.body)(session, &args).err_scopy(expression.span)
                    }
                    Value::UserFunction(function) => {
                        let params = session.get_params(function.params).to_vec();
                        let fn_env = function.env.child();

                        if args.len() != params.len() {
                            return Err(RuntimeError::UnexpectedParamCount.scopy(expression.span));
                        }

                        for (name, value) in params.into_iter().zip(args.into_iter()) {
                            fn_env.def(name, value).err_scopy(expression.span)?;
                        }

                        self.evaluate_expression(session, &fn_env, function.body)
                    }
                    _ => Err(RuntimeError::NotAFunction.scopy(expression.span)),
                }
            }
            Expression::If { cond, t_branch, f_branch } => {
                if self.evaluate_expression(session, env, cond)?.is_truthy() {
                    self.evaluate_expression(session, env, t_branch)
                } else {
                    self.evaluate_expression(session, env, f_branch)
                }
            }
            Expression::Fn { params, body } => {
                Ok(Value::UserFunction(Rc::new(UserFunction { params, body, env: env.clone() })))
            }
            Expression::Def { name, value } => {
                let value = self.evaluate_expression(session, env, value)?;
                env.def(name, value).err_scopy(expression.span)?;
                Ok(Value::Null)
            }
            Expression::Set { name, value } => {
                let value = self.evaluate_expression(session, env, value)?;
                env.set(name, value).err_scopy(expression.span)?;
                Ok(Value::Null)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            frontend::{parser::Parser, reader::Reader},
            runtime::builtins::make_builtins,
        },
    };

    fn eval_str_to_value(source: &str) -> Result<Value, SRuntimeError> {
        let mut session = Session::new();
        let roots = Parser::new().parse(&mut session, &mut Reader::new(source))?;
        let evaluator = Evaluator::new(&mut session, make_builtins()).err_sbetween(0, 1)?;
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
