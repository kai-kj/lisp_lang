use crate::{
    lowering::expression::{Expression, SExpression},
    runtime::{
        error::{RuntimeError, SRuntimeError},
        value::{BuiltinFunction, UserFunction, Value},
    },
    span::SpannedExt,
    symbol::{SymbolId, SymbolTable},
};

pub struct Interpreter<'table> {
    symbol_table: &'table mut SymbolTable,
    root_env: Environment<'table>,
}

impl<'table> Interpreter<'table> {
    pub fn new(
        symbol_table: &'table mut SymbolTable,
        builtins: &[(&'static str, BuiltinFunction)],
    ) -> Result<Self, SRuntimeError> {
        let mut interpreter = Self { symbol_table, root_env: Environment::new() };

        for builtin in builtins {
            interpreter
                .root_env
                .set(
                    &interpreter.symbol_table.add_symbol(&builtin.0),
                    Value::BuiltinFunction(builtin.1),
                )
                .map_err(|e| e.span_none())?;
        }

        Ok(interpreter)
    }

    pub fn interpret(&mut self, expression: &[SExpression]) -> Result<Value, SRuntimeError> {
        let mut result = Value::Null;
        for expression in expression {
            result = Self::interpret_expression(expression, &mut self.root_env)?;
        }
        Ok(result)
    }

    fn interpret_expression(
        expression: &SExpression,
        env: &mut Environment,
    ) -> Result<Value, SRuntimeError> {
        match &expression.value {
            Expression::Literal(v) => Ok(v.clone()),
            Expression::Variable(v) => {
                env.get(v).map(|v| v.clone()).map_err(|e| e.span(expression.span))
            }
            Expression::Call { call, args } => {
                let args = args
                    .iter()
                    .map(|a| Self::interpret_expression(a, &mut env.child()))
                    .collect::<Result<Vec<_>, _>>()?;

                match Self::interpret_expression(call, &mut env.child())? {
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

                        for (name, value) in function.args.iter().zip(args.iter()) {
                            function_env
                                .set(name, value.clone())
                                .map_err(|e| e.span(expression.span))?;
                        }

                        for expression in function.body.iter() {
                            result = Self::interpret_expression(expression, &mut function_env)?;
                        }

                        Ok(result)
                    }
                    _ => Err(RuntimeError::NotAFunction.span(expression.span)),
                }
            }
            Expression::If { cond, t_branch, f_branch } => {
                if Self::interpret_expression(cond, &mut env.child())?.is_truthy() {
                    Self::interpret_expression(t_branch, &mut env.child())
                } else {
                    Self::interpret_expression(f_branch, &mut env.child())
                }
            }
            Expression::Function { args, body } => {
                Ok(Value::UserFunction(UserFunction { args: args.clone(), body: body.clone() }))
            }
            Expression::Define { name, value } => {
                env.set(name, Self::interpret_expression(value, &mut env.child())?)
                    .map_err(|e| e.span(expression.span))?;
                Ok(Value::Null)
            }
        }
    }
}

pub struct Environment<'parent> {
    parent: Option<&'parent Environment<'parent>>,
    binds: std::collections::HashMap<SymbolId, Value>,
}

impl<'parent> Environment<'parent> {
    pub fn new() -> Self {
        Self { parent: None, binds: std::collections::HashMap::new() }
    }

    pub fn child(&'parent self) -> Self {
        Self { parent: Some(self), binds: std::collections::HashMap::new() }
    }

    pub fn set(&mut self, name: &SymbolId, value: Value) -> Result<(), RuntimeError> {
        if self.binds.contains_key(&name) {
            Err(RuntimeError::VariableAlreadyDefined)
        } else {
            self.binds.insert(*name, value);
            Ok(())
        }
    }

    pub fn get(&self, name: &SymbolId) -> Result<&Value, RuntimeError> {
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
            lowering::lowerer::Lowerer,
            parsing::parser::Parser,
            runtime::{builtins::make_builtins, error::SRuntimeError, value::Value},
        },
    };

    fn interpret_str(source: &str) -> Result<Value, SRuntimeError> {
        let mut symbol_table = SymbolTable::new();
        let mut lexer = Lexer::new(source);

        let mut parser = Parser::new(&mut symbol_table);
        let syntax_list = parser.parse(&mut lexer)?;

        let mut lowerer = Lowerer::new(&mut symbol_table);
        let expression_list = lowerer.lower(&syntax_list)?;

        let mut interpreter = Interpreter::new(&mut symbol_table, &make_builtins())?;
        let result = interpreter.interpret(&expression_list)?;

        Ok(result)
    }

    #[test]
    fn test_interpret_literal() {
        assert_eq!(
            interpret_str("1").unwrap(),
            Value::Integer(1),
        )
    }

    #[test]
    fn test_interpret_call_builtin() {
        assert_eq!(
            interpret_str("(+ 1 2)").unwrap(),
            Value::Integer(3),
        )
    }

    #[test]
    fn test_interpret_call_user() {
        assert_eq!(
            interpret_str("((fn (a b) (+ a b)) 1 2)").unwrap(),
            Value::Integer(3),
        )
    }

    #[test]
    fn test_interpret_if_a() {
        assert_eq!(
            interpret_str("(if true 4 2)").unwrap(),
            Value::Integer(4),
        )
    }

    #[test]
    fn test_interpret_if_b() {
        assert_eq!(
            interpret_str("(if false 4 2)").unwrap(),
            Value::Integer(2),
        )
    }
    
    #[test]
    fn test_interpret_define() {
        assert_eq!(
            interpret_str("(define add (fn (a b) (+ a b))) (add 1 2)").unwrap(),
            Value::Integer(3),
        )
    }
}
