# Source Surface

This file specifies the source subset implemented by the parser and AST.

## Grammar

```text
Module        ::= ModDecl? UseDecl* Item*
ModDecl       ::= "mod" ModuleName NL
UseDecl       ::= "use" ModuleName NL
Item          ::= Function | TestDecl
Function      ::= "pub"? "fn" Name "(" ParamList? ")" Return? Effects? NL
                  Contract*
                  BodyLine*
                  "end" NL?
TestDecl      ::= "test" Name "(" ")" Return Effects NL
                  Contract*
                  BodyLine*
                  "end" NL?
Param         ::= Name (":" TypeText)?
Return        ::= "->" ResultBinding? TypeText
ResultBinding ::= Name ":"
Effects       ::= "effects" "[" EffectList? "]"
Contract      ::= ("require" | "ensure") TextUntilNewline
BodyLine      ::= LetLine | ExprLine
LetLine       ::= "let" Name (":" TypeText)? "=" Expr NL
ExprLine      ::= Expr NL
```

`TypeText` is collected from source and parsed by the semantic type parser.
Contract predicates are collected as source text and validated by the contract
checker.

A return may name the returned value for postconditions with `-> name: Type`.
The binding is contract-facing only: it is visible to `ensure` clauses for the
same function, but not to `require` clauses, the function body, or callers.
Bare `result` has no special meaning.

`use` declarations create module import aliases. The current alias is the final
segment of the imported module path, so `use platform.io` declares the alias
`io`.

`test` is a top-level declaration keyword, not a visibility modifier. Test
declarations are selected by `veln test`, require an empty parameter list,
require an explicit return type and `effects [...]` clause, and are not ordinary
callable functions.

## Expressions

Implemented expressions:

- holes: `_` and `_name`, with optional `satisfy candidate => predicate`
- literals: strings, integers, floats, `true`, `false`, and `()`
- paths and calls: `name`, `module::name`, `callee(args...)`
- constructors: `Ok(value)`, `Err(error)`, `Some(value)`, and `None`
- prelude helpers as bare calls such as `list_len(items)`
- records: `{name: value, ...}`
- lists: `[value, ...]`
- prefix operators: `not`, `-`
- binary operators: `|>`, `or`, `and`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `+`,
  `-`, `*`, `/`
- postfix result propagation: `expr?`
- parenthesized expressions

A `satisfy` suffix is valid only on a hole expression. The suffix requires one
candidate binding, the `=>` separator, and a predicate. The candidate binding
is visible only inside the suffix predicate.

## Not Implemented

Implemented lowering and execution do not include `match`, user-defined ADT
declarations, dictionary literals, method calls, loops, mutation, classes,
traits, macros, comprehensions, anonymous functions, custom operators, package
manifests, foreign declarations, or doctest fences.
