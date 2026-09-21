use {
    crate::{
        expression::{Expression, ExpressionId, ExpressionRange},
        runtime::{
            error::{RuntimeError, SRuntimeError},
            session::{Environment, Session},
            value::{BuiltinFunction, UserFunction, Value},
        },
        span::SpannedExt,
    },
    std::rc::Rc,
};

pub struct Evaluator {
    root: Rc<Environment>,
}

impl Evaluator {
    pub fn new(
        session: &mut Session,
        builtins: &[(&'static str, BuiltinFunction)],
    ) -> Result<Self, RuntimeError> {
        let root = session.root_environment();

        for (name, builtin) in builtins {
            let name = session.push_symbol(name);
            let builtin = session.push_builtin_function(*builtin);
            root.set(name, Value::BuiltinFunction(builtin))?;
        }

        Ok(Self { root })
    }

    pub fn evaluate(
        &self,
        session: &mut Session,
        expressions: ExpressionRange,
    ) -> Result<Value, SRuntimeError> {
        let mut result = Value::Null;
        for expression in session.get_expressions(expressions).to_owned() {
            result = self.evaluate_expression(session, &self.root, expression)?;
        }
        Ok(result)
    }

    fn evaluate_expression(
        &self,
        session: &mut Session,
        environment: &Rc<Environment>,
        expression: ExpressionId,
    ) -> Result<Value, SRuntimeError> {
        let expression = session.get_expression(expression);

        match expression.value {
            Expression::Null => Ok(Value::Null),
            Expression::Boolean(v) => Ok(Value::Boolean(v)),
            Expression::Integer(v) => Ok(Value::Integer(v)),
            Expression::Float(v) => Ok(Value::Float(v)),
            Expression::String(v) => Ok(Value::String(v)),
            Expression::Symbol(v) => environment.get(v).map_err(|e| e.scopy(expression.span)),
            Expression::Do(expressions) => {
                let mut result = Value::Null;
                let expression = session.get_expressions(expressions).to_owned();
                for expression in expression {
                    result = self.evaluate_expression(session, environment, expression)?;
                }
                Ok(result)
            }
            Expression::Call { call, args } => {
                let call = self.evaluate_expression(session, environment, call)?;

                let args = session.get_expressions(args).to_owned();

                let args = args
                    .into_iter()
                    .map(|arg| self.evaluate_expression(session, environment, arg))
                    .collect::<Result<Vec<_>, _>>()?;

                match call {
                    Value::BuiltinFunction(function) => {
                        let function = session.get_builtin_function(function);
                        if args.len() != function.params.len() {
                            return Err(RuntimeError::UnexpectedParamCount.scopy(expression.span));
                        }
                        (function.body)(session, &args).map_err(|e| e.scopy(expression.span))
                    }
                    Value::UserFunction(function) => {
                        let function_env = session.child_environment(environment);

                        let function = session.get_user_function(function);
                        let params = session.get_params(function.params);

                        if args.len() != params.len() {
                            return Err(RuntimeError::UnexpectedParamCount.scopy(expression.span));
                        }

                        for (name, value) in params.iter().zip(args.into_iter()) {
                            function_env.set(*name, value).map_err(|e| e.scopy(expression.span))?;
                        }

                        self.evaluate_expression(session, &function_env, function.body)
                    }
                    _ => Err(RuntimeError::NotAFunction.scopy(expression.span)),
                }
            }
            Expression::If { cond, t_branch, f_branch } => {
                if self.evaluate_expression(session, environment, cond)?.is_truthy(session) {
                    self.evaluate_expression(session, environment, t_branch)
                } else {
                    self.evaluate_expression(session, environment, f_branch)
                }
            }
            Expression::Function { params, body } => {
                Ok(Value::UserFunction(session.push_user_function(UserFunction { params, body })))
            }
            Expression::Define { name, value } => {
                let value = self.evaluate_expression(session, environment, value)?;
                environment.set(name, value).map_err(|e| e.scopy(expression.span))?;
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
        let evaluator =
            Evaluator::new(&mut session, &make_builtins()).map_err(|e| e.sbetween(0, 1))?;
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
            eval_str_to_value("(def add (fn (a b) (+ a b))) (add 1 2)").unwrap(),
            Value::Integer(3)
        )
    }
}
