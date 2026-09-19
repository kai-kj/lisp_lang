mod lexing;
// mod lowering;
mod parsing;
// mod runtime;
mod span;
mod util;

// use crate::{
//     lexing::lexer::Lexer,
//     lowering::lowerer::lower,
//     parsing::parser::parse,
//     runtime::{builtins::make_builtins, interpreter::Interpreter},
// };
//
// fn main() {
//     // let source = r#"(- 1 (+ 2 3) 4)   (if true 4 2)  (fn (a b) (print a) (+ a b)) (define one 1)"#;
//
//     let source = r#"(+ 1 2)"#;
//
//     println!("\nSOURCE:");
//     println!("{}", source);
//
//     let mut lexer = Lexer::new(source);
//     let syntax_list = parse(&mut lexer).unwrap();
//
//     println!("\nSYNTAX - flat:");
//     for syntax in &syntax_list {
//         println!("{}", syntax);
//     }
//
//     println!("\nSYNTAX - indented:");
//     for syntax in &syntax_list {
//         println!("{}", syntax);
//     }
//
//     let expression_list = lower(&syntax_list).unwrap();
//
//     println!("\nEXPRESSION - flat:");
//     for expression in &expression_list {
//         println!("{}", expression);
//     }
//
//     println!("\nEXPRESSION - indented:");
//     for expression in &expression_list {
//         println!("{}", expression);
//     }
//
//     let mut interpreter = Interpreter::new(&make_builtins()).unwrap();
//     let result = interpreter.interpret(expression_list).unwrap();
//
//     println!("\nRESULT: {}", result);
// }

use {lexing::lexer::Lexer, parsing::parser::Parser};

fn main() {
    let source = r#"(- 1 (+ 2 3) 4)   (if true 4 2)  (fn (a b) (print a) (+ a b)) (define one 1)"#;
    let mut lexer = Lexer::new(source);
    let parser = Parser::new();
    let syntax_tree = parser.parse(&mut lexer).unwrap();
    println!("{:?}", syntax_tree);
}
