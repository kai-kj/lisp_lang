# Lisp

## General structure

- Reader: text -> lisp events
- Parser: lisp events -> AST
- Expander: expand macros
- Evaluator: AST -> value
- Compiler: AST -> bytecode
- VM: bytecode -> value

```
- frontend/
  - event.rs
  - reader.rs
  - parser.rs
- middleend/
  - expander.rs
  - optimizer.rs
  - type_checker.rs
- backend/
  - evaluator/
  - bytecode/
  - native/
- expression.rs
- util.rs
- span.rs


```