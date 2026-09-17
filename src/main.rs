mod lexing;
mod lowering;
mod parsing;
mod prelude;
mod span;
mod symbol;
mod value;

use crate::prelude::*;

fn main() {
    let source =
        r#"(- 1 (+ 2 3) 4)   (if true 4 2)  (lambda (a b) (print a) (+ a b)) (define one 1)"#;

    println!("\nSOURCE:");
    println!("{}", source);

    let mut symbol_table = SymbolTable::new();
    let mut parser = Parser::new(source);
    let lowerer = Lowerer::new();

    let syntax_list = parser.parse(&mut symbol_table).unwrap();

    println!("\nSYNTAX - flat:");
    for syntax in &syntax_list {
        println!("{}", syntax.with_symbols(&symbol_table));
    }

    println!("\nSYNTAX - indented:");
    for syntax in &syntax_list {
        println!("{}", syntax.with_symbols(&symbol_table).set_indent(2));
    }

    let expression_list = lowerer.lower(&mut symbol_table, &syntax_list).unwrap();

    println!("\nEXPRESSION - flat:");
    for expression in &expression_list {
        println!("{}", expression.with_symbols(&symbol_table));
    }

    println!("\nEXPRESSION - indented:");
    for expression in &expression_list {
        println!("{}", expression.with_symbols(&symbol_table).set_indent(2));
    }
}
