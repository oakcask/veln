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
Return        ::= "->" TypeText
Effects       ::= "effects" "[" EffectList? "]"
Contract      ::= ("require" | "ensure") TextUntilNewline
BodyLine      ::= LetLine | ExprLine
LetLine       ::= "let" Name (":" TypeText)? "=" Expr NL
ExprLine      ::= Expr NL
```

`TypeText` is collected from source and parsed by the semantic type parser.
Contract predicates are collected as source text and validated by the contract
checker.

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
- records: `{name: value, ...}`
- lists: `[value, ...]`
- prefix operators: `not`, `-`
- binary operators: `|>`, `or`, `and`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `+`,
  `-`, `*`, `/`
- postfix result propagation: `expr?`
- parenthesized expressions

## Not Implemented

Implemented lowering and execution do not include `match`, user-defined ADT
declarations, dictionary literals, method calls, loops, mutation, classes,
traits, macros, comprehensions, anonymous functions, custom operators, package
manifests, foreign declarations, or doctest fences.
