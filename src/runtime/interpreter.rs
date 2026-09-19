use crate::{
    lowering::expression::{Expression, SExpression},
    runtime::{
        error::{RuntimeError, SRuntimeError},
        value::{BuiltinFunction, UserFunction, Value},
    },
    span::SpannedExt,
};

pub struct Interpreter<'e, 's> {
    root_env: Environment<'e, 's>,
}

impl<'e, 's> Interpreter<'e, 's> {
    pub fn new(builtins: &[(&'static str, BuiltinFunction<'s>)]) -> Result<Self, SRuntimeError> {
        let mut interpreter = Self { root_env: Environment::new() };

        for builtin in builtins {
            interpreter
                .root_env
                .set(&builtin.0, Value::BuiltinFunction(builtin.1))
                .map_err(|e| e.span_none())?;
        }

        Ok(interpreter)
    }

    pub fn interpret(
        &mut self,
        expression: Vec<SExpression<'s>>,
    ) -> Result<Value<'s>, SRuntimeError> {
        let mut result = Value::Null;
        for expression in expression {
            result = Self::interpret_expression(expression, &mut self.root_env)?;
        }
        Ok(result)
    }

    fn interpret_expression(
        expression: SExpression<'s>,
        env: &mut Environment<'_, 's>,
    ) -> Result<Value<'s>, SRuntimeError> {
        match expression.value {
            Expression::Literal(v) => Ok(v),
            Expression::Symbol(v) => {
                env.get(v).map(|v| v.clone()).map_err(|e| e.span(expression.span))
            }
            Expression::Call { call, args } => {
                let call = Self::interpret_expression(*call, env)?;

                let args = args
                    .into_iter()
                    .map(|a| Self::interpret_expression(a, env))
                    .collect::<Result<Vec<_>, _>>()?;

                match call {
                    Value::BuiltinFunction(function) => {
                        if args.len() != function.args {
                            return Err(RuntimeError::InvalidArgCount.span(expression.span));
                        }
                        (function.body)(&args).map_err(|e| e.span(expression.span))
                    }
                    Value::UserFunction(function) => {
                        if args.len() != function.args.len() {
                            return Err(RuntimeError::InvalidArgCount.span(expression.span));
                        }

                        let mut function_env = env.child();
                        let mut result = Value::Null;

                        for (name, value) in function.args.iter().zip(args.into_iter()) {
                            function_env.set(name, value).map_err(|e| e.span(expression.span))?;
                        }

                        for expression in function.body.into_iter() {
                            result = Self::interpret_expression(expression, &mut function_env)?;
                        }

                        Ok(result)
                    }
                    _ => Err(RuntimeError::NotAFunction.span(expression.span)),
                }
            }
            Expression::If { cond, t_branch, f_branch } => {
                if Self::interpret_expression(*cond, env)?.is_truthy() {
                    Self::interpret_expression(*t_branch, env)
                } else {
                    Self::interpret_expression(*f_branch, env)
                }
            }
            Expression::Function { args, body } => {
                Ok(Value::UserFunction(UserFunction { args, body }))
            }
            Expression::Define { name, value } => {
                let value = Self::interpret_expression(*value, env)?;
                env.set(name, value).map_err(|e| e.span(expression.span))?;
                Ok(Value::Null)
            }
        }
    }
}

pub struct Environment<'e, 's> {
    parent: Option<&'e Environment<'e, 's>>,
    binds: std::collections::HashMap<&'s str, Value<'s>>,
}

impl<'e, 's> Environment<'e, 's> {
    pub fn new() -> Self {
        Self { parent: None, binds: std::collections::HashMap::new() }
    }

    pub fn child(&'e self) -> Self {
        Self { parent: Some(self), binds: std::collections::HashMap::new() }
    }

    pub fn set(&mut self, name: &'s str, value: Value<'s>) -> Result<(), RuntimeError> {
        if self.binds.contains_key(&name) {
            Err(RuntimeError::VariableAlreadyDefined)
        } else {
            self.binds.insert(name, value);
            Ok(())
        }
    }

    pub fn get(&self, name: &'s str) -> Result<&Value<'s>, RuntimeError> {
        if let Some(v) = self.binds.get(&name) {
            Ok(v)
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            Err(RuntimeError::VariableNotDefined)
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            lexing::lexer::Lexer,
            lowering::lowerer::lower,
            parsing::parser::parse,
            runtime::{builtins::make_builtins, error::SRuntimeError, value::Value},
        },
    };

    fn interpret_str(source: &str) -> Result<Value, SRuntimeError> {
        let mut lexer = Lexer::new(source);
        let syntax_list = parse(&mut lexer)?;
        let expression_list = lower(&syntax_list)?;

        let mut interpreter = Interpreter::new(&make_builtins())?;
        let result = interpreter.interpret(expression_list)?;

        Ok(result)
    }

    #[test]
    fn test_interpret_literal() {
        assert_eq!(interpret_str("1").unwrap(), Value::Integer(1),)
    }

    #[test]
    fn test_interpret_call_builtin() {
        assert_eq!(interpret_str("(+ 1 2)").unwrap(), Value::Integer(3),)
    }

    #[test]
    fn test_interpret_call_user() {
        assert_eq!(interpret_str("((fn (a b) (+ a b)) 1 2)").unwrap(), Value::Integer(3),)
    }

    #[test]
    fn test_interpret_if_a() {
        assert_eq!(interpret_str("(if true 4 2)").unwrap(), Value::Integer(4),)
    }

    #[test]
    fn test_interpret_if_b() {
        assert_eq!(interpret_str("(if false 4 2)").unwrap(), Value::Integer(2),)
    }

    #[test]
    fn test_interpret_define() {
        assert_eq!(
            interpret_str("(def add (fn (a b) (+ a b))) (add 1 2)").unwrap(),
            Value::Integer(3),
        )
    }
}
