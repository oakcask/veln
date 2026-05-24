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
Contract      ::= ("require" | "ensure") ContractPredicate NL
BodyLine      ::= LetLine | ExprLine
LetLine       ::= "let" Name (":" TypeText)? "=" Expr NL
ExprLine      ::= Expr NL
Record        ::= "{" (Name ":" Expr) ("," Name ":" Expr)* ","? "}"
Dict          ::= "{" Expr ":" Expr ("," Expr ":" Expr)* ","? "}"
Match         ::= "match" Expr NL MatchArm+ "end"
MatchArm      ::= Pattern "=>" Expr NL
Pattern       ::= "_" | BindingName | Literal | ConstructorPattern | RecordPattern
ConstructorPattern ::= ConstructorName "(" PatternList? ")"
                     | ConstructorName
RecordPattern ::= "{" PatternFieldList? "}"
PatternList   ::= Pattern ("," Pattern)* ","?
PatternFieldList ::= PatternField ("," PatternField)* ","?
PatternField  ::= Name ":" Pattern
```

`TypeText` is collected from source and parsed by the semantic type parser.
Contract predicates parse through a narrower predicate production before
semantic contract validation.

A return may name the returned value for postconditions with `-> name: Type`.
The binding is contract-facing only: it is visible to `ensure` clauses for the
same function, but not to `require` clauses, the function body, or callers.
Bare `result` has no special meaning.

`mod` declares the source module identity. The header is optional for a
single-file source with no imports. A source file with one or more `use`
declarations must declare `mod` before those imports.

`use` declarations create module import aliases. The current alias is the final
segment of the imported module path, so `use platform.io` declares the alias
`io`.

Public `fn` declarations are the implemented public API boundary. Dedicated
export lists are not implemented.

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
- dictionaries: `{key_expr: value_expr, ...}` when the first entry key is not
  an identifier
- record field access: `expr.name`
- lists: `[value, ...]`
- match expressions over literals, bindings, `_`, record patterns, and built-in
  constructors `Some`, `None`, `Ok`, and `Err`
- prefix operators: `not`, `-`
- pipelines: `expr |> target(args...)`
- binary operators: `or`, `and`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `+`, `-`,
  `*`, `/`
- postfix result propagation: `expr?`
- parenthesized expressions

A `satisfy` suffix is valid only on a hole expression. The suffix requires one
candidate binding, the `=>` separator, and a predicate. The candidate binding
is visible only inside the suffix predicate.

Pipelines require a call expression on the right. The piped expression is
inserted as the first argument of that call, so `value |> target(extra)` is
checked and executed as `target(value, extra)`. A non-call pipeline target
reports `type.pipeline_target`.

`match` arms are tried in source order. The implemented pattern subset covers
wildcard `_`, binding names, literals, record patterns, and the built-in
constructors `Some`, `None`, `Ok`, and `Err`. Record patterns match when the
scrutinee is a record containing every named pattern field and every nested
field pattern matches. Exhaustiveness is not statically checked in the current
slice.

## Contract Predicates

`require`, `ensure`, and hole `satisfy` predicates accept this implemented
syntax:

- literals, names, qualified names, and `()`
- grouping with parentheses
- plain or qualified call syntax
- field access syntax
- prefix `not` and `-`
- binary `or`, `and`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `+`, `-`, `*`, and
  `/`

The parser rejects holes, `?`, pipelines, `match`, records, and lists in
contract predicates before semantic checking. A syntactically valid predicate
may still fail contract validation. Function calls must resolve to discovered
pure functions, call arguments must be assignable to declared parameter types,
and field access must resolve through record-typed values visible to the
clause.

## Not Implemented

Implemented lowering and execution do not include user-defined ADT
declarations, method calls, loops, mutation, classes, traits, macros,
comprehensions, anonymous functions, custom operators, package manifests,
foreign declarations, or doctest fences.
