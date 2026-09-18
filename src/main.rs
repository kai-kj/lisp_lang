mod display;
mod lexing;
mod lowering;
mod parsing;
mod runtime;
mod span;
mod symbol;

use crate::{
    display::WithDisplayContextExt,
    lexing::lexer::Lexer,
    lowering::lowerer::Lowerer,
    parsing::parser::Parser,
    runtime::{builtins::make_builtins, interpreter::Interpreter},
    symbol::SymbolTable,
};

fn main() {
    // let source = r#"(- 1 (+ 2 3) 4)   (if true 4 2)  (fn (a b) (print a) (+ a b)) (define one 1)"#;

    let source = r#"(+ 1 2)"#;

    println!("\nSOURCE:");
    println!("{}", source);

    let mut symbol_table = SymbolTable::new();
    let mut lexer = Lexer::new(source);

    let mut parser = Parser::new(&mut symbol_table);
    let syntax_list = parser.parse(&mut lexer).unwrap();

    println!("\nSYNTAX - flat:");
    for syntax in &syntax_list {
        println!("{}", syntax.with_symbols(&symbol_table));
    }

    println!("\nSYNTAX - indented:");
    for syntax in &syntax_list {
        println!("{}", syntax.with_symbols(&symbol_table).set_indent(2));
    }

    let mut lowerer = Lowerer::new(&mut symbol_table);
    let expression_list = lowerer.lower(&syntax_list).unwrap();

    println!("\nEXPRESSION - flat:");
    for expression in &expression_list {
        println!("{}", expression.with_symbols(&symbol_table));
    }

    println!("\nEXPRESSION - indented:");
    for expression in &expression_list {
        println!("{}", expression.with_symbols(&symbol_table).set_indent(2));
    }

    let mut interpreter = Interpreter::new(&mut symbol_table, &make_builtins()).unwrap();
    let result = interpreter.interpret(&expression_list).unwrap();

    println!("\nRESULT: {}", result);
}
